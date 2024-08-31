#![feature(generic_arg_infer)]
use axiom_sdk::axiom::{AxiomAPI, AxiomComputeFn, AxiomResult};
use axiom_sdk::cmd::run_cli;
use axiom_sdk::Fr;
use input::{VolatilityCircuitInput, VolatilityInput};
use volatility::VolatilityChip;

use halo2_base::AssignedValue;
use std::fs::File;
use std::io::BufReader;

mod fixed;
mod volatility; 
mod utils;
mod input;

const PRECISION: u32 = 48;
const SAMPLE_SIZE: usize = 8192;
const FILE:&str = "data/inputs.json";

impl AxiomComputeFn for VolatilityInput<PRECISION,SAMPLE_SIZE> {
    fn compute(
        api: &mut AxiomAPI,
        input: VolatilityCircuitInput<AssignedValue<Fr>,PRECISION,SAMPLE_SIZE>,
    ) -> Vec<AxiomResult> {

        let chip:VolatilityChip<Fr,PRECISION> = VolatilityChip::new(&api.builder.base);

        let values =  input.0;

        let ctx = api.ctx();
        
        let volatility = chip.volatility(ctx, values);
        
        let value = chip.dequantization(*volatility.value());

        println!("Axiom    : {}",value);

        vec![
            volatility.into()
        ]
    }
}

fn main() {

    env_logger::init();

    let input:VolatilityInput<PRECISION,SAMPLE_SIZE> = File::open(FILE)
    .map(|file| BufReader::new(file))
    .map(|reader| serde_json::from_reader(reader).expect("Invalid JSON"))
    .expect("Input file can not be read");

    let ticks = input.ticks;

    println!("\x1b[93mNumber of ticks: {}\x1b[0m",ticks.len());

    let volatility_optmized = utils::calculate_optimized(&ticks);
    let volatility_original = utils::calculate_original(&ticks);

    println!("\x1b[93mVolatility:\x1b[0m");
    println!("Reference: {}",volatility_original);
    println!("Optimized: {}",volatility_optmized);

    run_cli::<VolatilityInput<PRECISION,SAMPLE_SIZE> >();
}