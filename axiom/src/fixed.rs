// Semiotic
// Fixed point number conversion and constants for the computation.

use crate::utils::ScalarFieldExt;
use halo2_base::utils::{biguint_to_fe, BigPrimeField};
use num_bigint::BigUint;
use std::ops::Sub;

pub(crate) struct FixedPointConstants<F: BigPrimeField, const PRECISION_BITS: u32> {
    pub quantization_scale: F,
    pub bn254_max: F,
    pub negative_point: F,
}

impl<F: BigPrimeField, const PRECISION_BITS: u32> FixedPointConstants<F, PRECISION_BITS> {
    pub fn quantization(&self, value: f64) -> F {
        let sign = value.signum();
        let x = value.abs();
        let x_q = (x * self.quantization_scale.get_lower_64() as f64).round() as u128;
        let x_q_biguint = BigUint::from(x_q).to_bytes_le();
        let mut x_q_bytes_le = [0u8; 64];
        for (idx, val) in x_q_biguint.iter().enumerate() {
            x_q_bytes_le[idx] = *val;
        }
        let mut x_q_f = F::from_uniform_bytes(&x_q_bytes_le);

        if sign < 0.0 {
            x_q_f = self.bn254_max - x_q_f + F::ONE;
        }

        x_q_f
    }

    pub fn dequantization(&self, value: F) -> f64 {
        let mut x_mut = value;
        let negative = if value > self.negative_point {
            x_mut = self.bn254_max - value - F::ONE;
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
}

impl<F: BigPrimeField, const PRECISION_BITS: u32> Default
    for FixedPointConstants<F, PRECISION_BITS>
{
    fn default() -> Self {
        assert!(PRECISION_BITS <= 63u32, "support only precision bits <= 63");
        assert!(PRECISION_BITS >= 32u32, "support only precision bits >= 32");

        // Simple uniform symmetric quantization scheme which enforces zero point to be exactly 0
        // to reduce lots of computations.
        // Quantization: x_q = xS where S is `quantization_scale`
        // De-quantization: x = x_q / S
        let quantization_scale = F::from_u128(2u128.pow(PRECISION_BITS));
        // Becuase BN254 is cyclic, negative number will be denoted as (-x) % m = m - x where m = 2^254,
        // in this chip, we treat all x > negative_point as a negative numbers.
        let bn254_max = biguint_to_fe(
            &BigUint::parse_bytes(&F::MODULUS[2..].bytes().collect::<Vec<u8>>(), 16)
                .unwrap()
                .sub(1u32),
        );
        // -max_value % m = negative_point
        let negative_point = bn254_max - F::from_u128(2u128.pow(PRECISION_BITS * 2 + 1)) + F::ONE;

        Self {
            quantization_scale,
            bn254_max,
            negative_point,
        }
    }
}
