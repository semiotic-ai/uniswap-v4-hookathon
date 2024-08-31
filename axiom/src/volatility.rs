use crate::fixed::FixedPointConstants;
use halo2_base::{
    gates::{circuit::builder::BaseCircuitBuilder, GateInstructions, RangeChip},
    utils::{biguint_to_fe, fe_to_biguint, BigPrimeField},
    AssignedValue, Context, QuantumCell,
    QuantumCell::{Constant, Existing, Witness},
};
use num_integer::Integer;
use std::iter;

pub struct VolatilityChip<F: BigPrimeField, const PRECISION_BITS: u32 = 32> {
    range: RangeChip<F>,
    constants: FixedPointConstants<F, PRECISION_BITS>,
}

impl<F: BigPrimeField, const PRECISION_BITS: u32> VolatilityChip<F, PRECISION_BITS> {
    pub fn new(builder: &BaseCircuitBuilder<F>) -> Self {
        Self {
            range: builder.range_chip(),
            constants: FixedPointConstants::<F, PRECISION_BITS>::default(),
        }
    }

    pub fn dequantization(&self, value: F) -> f64 {
        self.constants.dequantization(value)
    }

    pub fn quantization(&self, value: f64) -> F {
        self.constants.quantization(value)
    }

    /// Optimized to scale a unsigned value to precision
    fn scale(
        &self,
        ctx: &mut Context<F>,
        a: impl Into<QuantumCell<F>>,
    ) -> (AssignedValue<F>, AssignedValue<F>) {
        let a: QuantumCell<F> = a.into();

        let b = fe_to_biguint(&self.constants.quantization_scale);

        let (q, r) = fe_to_biguint(a.value()).div_mod_floor(&b);

        ctx.assign_region(
            [
                Witness(biguint_to_fe(&r)),
                Constant(biguint_to_fe(&b)),
                Witness(biguint_to_fe(&q)),
                a,
            ],
            [0],
        );

        (ctx.get(-2), ctx.get(-4))
    }

    /// Optimized to to multiply two unsigned values
    fn mul(
        &self,
        ctx: &mut Context<F>,
        a: impl Into<QuantumCell<F>>,
        b: impl Into<QuantumCell<F>>,
    ) -> AssignedValue<F> {
        let a = a.into();
        let b = b.into();

        let ab = self.range.gate.mul(ctx, a, b);
        let (res, _) = self.scale(ctx, ab);

        res
    }

    fn sub(
        &self,
        ctx: &mut Context<F>,
        a: impl Into<QuantumCell<F>>,
        b: impl Into<QuantumCell<F>>,
    ) -> AssignedValue<F>
    where
        F: BigPrimeField,
    {
        self.range.gate.sub(ctx, a, b)
    }

    /// Calculates the volatility square of the provided values
    pub fn volatility<QA>(
        &self,
        ctx: &mut Context<F>,
        a: impl IntoIterator<Item = QA>,
    ) -> AssignedValue<F>
    where
        QA: Into<QuantumCell<F>>,
    {
        let row_offset = ctx.advice.len();

        let mut a = a.into_iter().peekable();

        let previous = a.next();

        if previous.is_none() {
            return ctx.load_zero();
        }

        let mut previous_value: QuantumCell<F> = previous.unwrap().into();

        if a.peek().is_none() {
            return ctx.load_zero();
        }

        // Below iteration compresses deviation calculations into n-2 cells compared to
        // standard methods per step/item. [n0,1,n1-n0,n1,1,n2-n1,n2...] to comply axioms
        // s*(a+b.c-d) = 0 circuit.
        let cells = iter::once(previous_value)
            .chain(a.flat_map(|current| {
                let current_value: QuantumCell<F> = current.into();
                let delta_value = *current_value.value() - previous_value.value();
                previous_value = current_value;
                [Constant(F::ONE), Witness(delta_value), current_value]
            }))
            .collect::<Vec<QuantumCell<F>>>();

        let len = cells.len() - 1;

        let last_value = ctx.assign_region_last(cells, (0..len).step_by(3).map(|i| i as isize));

        let first_value = ctx.get(row_offset as isize);

        // Calculate sum of deviations which is simply difference between the last and
        // first item. (n1-n0) + (n2-n1) + ... + (nn-nn-1) = nn-n0
        let delta_value = *last_value.value() - *first_value.value();

        ctx.assign_region(
            [
                Existing(first_value),
                Constant(F::ONE),
                Witness(delta_value),
                Existing(last_value),
            ],
            [0],
        );

        let delta_value = ctx.get(-2);

        let delta_sum_sq = *delta_value.value() * delta_value.value();

        let delta_sum_sq = ctx.assign_region_last(
            [
                Constant(F::ZERO),
                Existing(delta_value),
                Existing(delta_value),
                Witness(delta_sum_sq),
            ],
            [0],
        );

        let mut delta_sq_sum = F::ZERO;

        // Calculate sum of squares of deviations which is (n1-n0)^2 + (n2-n1)^2 + ...
        // + (nn-nn-1)^. Again we use a similar compression above with same axiom circuit.
        // [0,n1-n0,n1-n0,(n1-n0)^2,n2-n1,n2-n1,(n2-n1)^2+(n1-n0)^2,n3-n2,n3-n2,(n3-n2)^2+(n2-n1)^2+(n1-n0)^2...]
        let cells = iter::once(Constant(F::ZERO))
            .chain(
                (0..len)
                    .step_by(3)
                    .map(|i| ctx.get((row_offset + i + 2) as isize))
                    .flat_map(|delta| {
                        let delta_value: QuantumCell<F> = delta.into();
                        delta_sq_sum += *delta_value.value() * delta_value.value();
                        [delta_value, delta_value, Witness(delta_sq_sum)]
                    }),
            )
            .collect::<Vec<QuantumCell<F>>>();

        let delta_sq_sum = ctx.assign_region_last(cells, (0..len).step_by(3).map(|i| i as isize));

        // As we are sure both delta_sum_sq and delta_sq_sum are positive, we can safely
        // scale them to precision.
        let delta_sq_sum = self.scale(ctx, delta_sq_sum).0;

        let delta_sum_sq = self.scale(ctx, delta_sum_sq).0;

        let len = ((len / 3) + 1) as f64;

        let n_inv = ctx.load_constant(self.quantization(1f64 / len));
        let n1_inv = ctx.load_constant(self.quantization(1f64 / (len - 1f64)));

        // Again all values are positive, we can safely use unsigned multiplication.
        let delta_sum_sq_div_n = self.mul(ctx, delta_sum_sq, n_inv);

        let delta = self.sub(ctx, delta_sq_sum, delta_sum_sq_div_n);

        self.mul(ctx, delta, n1_inv)
    }
}
