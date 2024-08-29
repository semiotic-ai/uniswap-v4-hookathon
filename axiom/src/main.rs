
use axiom_sdk::axiom::{AxiomAPI, AxiomComputeFn, AxiomComputeInput, AxiomResult};
use axiom_sdk::cmd::run_cli;
use axiom_sdk::axiom_circuit;
use axiom_sdk::Fr;
use volatility::VolatilityChip;

use halo2_base::AssignedValue;
use std::fs::File;
use std::io::{BufRead, BufReader};
use halo2_base::QuantumCell::Constant;

mod volatility; 
mod utils;

const PRECISION: u32 = 48;
const FILE:&str = "data/ticks_8192.csv";


#[AxiomComputeInput]
pub struct VolatilityInput {
    pub dummy:u32,
}

impl AxiomComputeFn for VolatilityInput {
    fn compute(
        api: &mut AxiomAPI,
        _: VolatilityCircuitInput<AssignedValue<Fr>>,
    ) -> Vec<AxiomResult> {

        let ticks = File::open(FILE)
        .map(|file| BufReader::new(file))
        .map(|reader| reader.lines())
        .expect("Ticks file can not be read")
        .skip(1)
        .map(|line| 
            line.map(|value|
                str::parse::<f64>(&value).expect("Can not parse value")
            ).expect("Can not read line")
        ).collect::<Vec<_>>();

        println!("\x1b[93mNumber of ticks: {}\x1b[0m",ticks.len());

        let volatility_optmized = utils::calculate_optimized(&ticks);
        let volatility_original = utils::calculate_original(&ticks);

        let chip:VolatilityChip<Fr,PRECISION> = VolatilityChip::new(&api.builder.base);

        let values = ticks
            .into_iter()
            .map(|value| Constant(chip.quantization(value)))
            .collect::<Vec<_>>();

        let ctx = api.ctx();
        
        let volatility = chip.volatility(ctx, values);
        
        let value = chip.dequantization(*volatility.value());

        println!("\x1b[93mVolatility:\x1b[0m");
        println!("Reference: {}",volatility_original);
        println!("Optimized: {}",volatility_optmized);
        println!("Axiom    : {}",value);

        vec![
            volatility.into()
        ]
    }
}


fn main() {
    env_logger::init();
    run_cli::<VolatilityInput>();
}