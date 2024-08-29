use halo2_base::{gates::{circuit::builder::BaseCircuitBuilder, GateInstructions, RangeChip}, utils::{biguint_to_fe, fe_to_biguint, BigPrimeField}, AssignedValue, Context, QuantumCell};
use halo2_base::QuantumCell::{Constant, Existing, Witness};
use num_bigint::BigUint;
use crate::utils::ScalarFieldExt;
use core::ops::Sub;
use std::iter;
use num_integer::Integer;



pub struct VolatilityChip<F: BigPrimeField, const PRECISION_BITS: u32 = 32> {
    range: RangeChip<F>,
    quantization_scale:F,
    bn254_max:F,
    negative_point:F,
}


impl<F: BigPrimeField, const PRECISION_BITS: u32> VolatilityChip<F, PRECISION_BITS> {

    pub fn new(builder: &BaseCircuitBuilder<F>) -> Self {
        assert!(PRECISION_BITS <= 63, "support only precision bits <= 63");
        assert!(PRECISION_BITS >= 32, "support only precision bits >= 32");
        let range = builder.range_chip();
        let quantization_scale = F::from_u128(2u128.pow(PRECISION_BITS as u32));
        let bn254_max = biguint_to_fe(&BigUint::parse_bytes(
            &F::MODULUS[2..].bytes().collect::<Vec<u8>>(), 16).unwrap().sub(1u32));
        let negative_point = bn254_max - F::from_u128(2u128.pow(PRECISION_BITS * 2 + 1)) + F::ONE;

        Self { 
            range,
            quantization_scale,
            bn254_max,
            negative_point,
        }
    }

    /// Covert from f64 to prime field
    pub fn quantization(&self, x: f64) -> F {

        let sign = x.signum();
        let x = x.abs();
        let x_q = (x * self.quantization_scale.get_lower_64() as f64).round() as u128;
        let x_q_biguint = BigUint::from(x_q).to_bytes_le();
        let mut x_q_bytes_le: [u8; 64] = [0u8; 64];
        for (idx, val) in x_q_biguint.iter().enumerate() {
            x_q_bytes_le[idx] = *val;
        }
        let mut x_q_f = F::from_uniform_bytes(&x_q_bytes_le);

        if sign < 0.0 {
            x_q_f = self.bn254_max - x_q_f + F::ONE;
        }

        x_q_f

    }

    /// Covert from prime field to f64
    pub fn dequantization(&self, x: F) -> f64 {
        let mut x_mut = x;
        let negative = if x > self.negative_point {
            x_mut = self.bn254_max - x - F::ONE;
            -1f64
        } else {
            1f64
        };
        let x_u128: u128 = x_mut.get_lower_128();
        let quantization_scale = self.quantization_scale.get_lower_128();
        let x_int = (x_u128 / quantization_scale) as f64;
        let x_frac = (x_u128 % quantization_scale) as f64 / quantization_scale as f64;
        let x_deq = negative * (x_int + x_frac);

        x_deq
    }


    /// Optimized to scale a unsigned value to precision
    fn scale(
        &self,
        ctx: &mut Context<F>,
        a: impl Into<QuantumCell<F>>
    ) -> (AssignedValue<F>, AssignedValue<F>) {

        let a:QuantumCell<F> = a.into();

        let b = fe_to_biguint(&self.quantization_scale);
        
        let (q,r) = fe_to_biguint(a.value()).div_mod_floor(&b);

        ctx.assign_region(
            [Witness(biguint_to_fe(&r)), Constant(biguint_to_fe(&b)), Witness(biguint_to_fe(&q)), a],
            [0]
        );

        (ctx.get(-2),ctx.get(-4))
    }

    /// Optimized to to multiply two unsigned values
    fn mul(
        &self,
        ctx: &mut Context<F>,
        a: impl Into<QuantumCell<F>>,
        b: impl Into<QuantumCell<F>>
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
        b: impl Into<QuantumCell<F>>
    ) -> AssignedValue<F>
    where 
        F: BigPrimeField
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
        QA: Into<QuantumCell<F>> {

            let row_offset = ctx.advice.len();

            let mut a = a.into_iter().peekable();

            let previous = a.next();

            if previous.is_none() {
                return ctx.load_zero();
            }
            
            let mut previous_value:QuantumCell<F> = previous.unwrap().into();
            
            if a.peek().is_none() {
                return ctx.load_zero();
            }
            
            let cells = iter::once(previous_value).chain(a.flat_map(|current| {
                let current_value:QuantumCell<F> = current.into();
                let delta_value = *current_value.value()-previous_value.value();
                previous_value = current_value;
                [Constant(F::ONE), Witness(delta_value),current_value]
            })).collect::<Vec<QuantumCell<F>>>();

            let len = cells.len() -1;

            let last_value = ctx.assign_region_last(cells, (0..len).step_by(3).map(|i| i as isize));

            let first_value = ctx.get(row_offset as isize);

            let delta_value = *last_value.value() - *first_value.value();
 
            ctx.assign_region([Existing(first_value),Constant(F::ONE),Witness(delta_value),Existing(last_value)],[0]);

            let delta_value = ctx.get(-2);

            let delta_sum_sq = *delta_value.value() * delta_value.value();

            let delta_sum_sq = ctx.assign_region_last([Constant(F::ZERO),Existing(delta_value),Existing(delta_value), Witness(delta_sum_sq)],[0]);

            let mut delta_sq_sum = F::ZERO;

            let cells = iter::once(Constant(F::ZERO)).chain(
                (0..len).step_by(3).map(|i| ctx.get((row_offset +  i + 2) as isize)).flat_map(|delta| {
                    let delta_value:QuantumCell<F> = delta.into();
                    delta_sq_sum += *delta_value.value() * delta_value .value();
                    [delta_value,delta_value,Witness(delta_sq_sum)]
                })).collect::<Vec<QuantumCell<F>>>();

            let delta_sq_sum = ctx.assign_region_last(cells, (0..len).step_by(3).map(|i| i as isize));
            
            let delta_sq_sum = self.scale(ctx,delta_sq_sum).0;

            let delta_sum_sq = self.scale(ctx,delta_sum_sq).0;

            let len = ((len/3)+1) as f64;

            let n_inv = ctx.load_constant(self.quantization(1f64/len));
            let n1_inv = ctx.load_constant(self.quantization(1f64/(len - 1f64)));
    
            let delta_sum_sq_div_n = self.mul(ctx,delta_sum_sq, n_inv);

            let delta = self.sub(ctx,delta_sq_sum,delta_sum_sq_div_n);
    
            self.mul(ctx,delta,n1_inv)

        }
    
}


