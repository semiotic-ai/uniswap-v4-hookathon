// Original derive macro `AxiomComputeInput` generates a lot of copy which requires a lot of memory allocation
// This custom implementation is used to avoid memory allocation error with large number of inputs (ie. 8192)

use anyhow::Result;
use axiom_sdk::{axiom::AxiomComputeInput, axiom_circuit::{axiom_eth::Field, input::flatten::InputFlatten}};
use serde::{Deserialize, Serialize};
use crate::fixed::FixedPointConstants;

#[derive(Clone, Debug,Default,Serialize, Deserialize)]
pub struct VolatilityInput<const PRECISION_BITS:u32,const N:usize>
{
    pub ticks: Vec<f64>
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VolatilityCircuitInput<T: Copy,const PRECISION_BITS:u32, const N: usize>(pub Vec<T>);


impl<T: Copy + Default, const PRECISION_BITS:u32, const N: usize> Default for VolatilityCircuitInput<T,PRECISION_BITS,N> {
    fn default() -> Self {
        Self(vec![T::default(); N])
    }
}

impl<T: Copy + Default,const PRECISION_BITS:u32, const N: usize> VolatilityCircuitInput<T,PRECISION_BITS,N> {
    pub fn new(vec: Vec<T>) -> anyhow::Result<Self> {
        if vec.len() != N {
            anyhow::bail!("Invalid input length: {} != {}", vec.len(), N);
        }
        Ok(VolatilityCircuitInput(vec))
    }

    pub fn into_inner(self) -> Vec<T> {
        self.0
    }
}

impl<F:Field,const PRECISION_BITS:u32,const N:usize> From<VolatilityInput<PRECISION_BITS,N>> for VolatilityCircuitInput<F,PRECISION_BITS,N> {
    fn from(input: VolatilityInput<PRECISION_BITS,N>) -> Self {
        let constants = FixedPointConstants::<F,PRECISION_BITS>::default();
        VolatilityCircuitInput(input.ticks.iter().map(|x| constants.quantization(*x)).collect())
    }
}

impl<T: Copy,const PRECISION_BITS:u32, const N: usize> InputFlatten<T> for VolatilityCircuitInput<T,PRECISION_BITS,N> {
    const NUM_FE: usize = N;
    fn flatten_vec(&self) -> Vec<T> {
        self.0.clone()
    }
    fn unflatten(vec: Vec<T>) -> Result<Self> {
        if vec.len() != Self::NUM_FE {
            anyhow::bail!(
                "Invalid input length: {} != {}",
                vec.len(),
                Self::NUM_FE
            );
        }
        Ok(VolatilityCircuitInput(vec))
    }
}

impl<const PRECISION_BITS:u32,const N:usize> AxiomComputeInput for VolatilityInput<PRECISION_BITS,N> {
     type LogicInput = VolatilityInput<PRECISION_BITS,N>;
     type Input<T: Copy> = VolatilityCircuitInput<T,PRECISION_BITS,N>;
}
