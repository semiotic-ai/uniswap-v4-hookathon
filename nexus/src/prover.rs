
use anyhow::{Result, anyhow, Context};
use nexus_sdk::compile::CompileOpts;
use nexus_sdk::nova::seq::*;
use nexus_sdk::*;
use views::UncheckedView;

use std::{fs::File, path::Path};
use std::io::Write;

use crate::common::*;
use crate::ticks::PublicData;

const PACKAGE_NAME: &str = "guest";

const DATA_FILE: &str = "src/guest/src/data.rs";

// Default zkVM memory limit in MB
const DEFAULT_MEMORY_LIMIT:usize = 8;

const PUBLIC_PARAMETERS_FILE: &str = "public_params.bin";

pub fn get_public_parameters() -> Result<PP> {

    println!("Setting up Nova public parameters...");

    let public_params_path = Path::new(PUBLIC_PARAMETERS_FILE);

    if public_params_path.exists() {
        println!("Public parameters file found. Loading...");
        PP::load(public_params_path).context("failed to load parameters")
    }
    else {
        println!("Public parameters file not found. Generating...");
        let pp = PP::generate().context("failed to generate parameters")?;
        PP::save(&pp,public_params_path).context("failed to save parameters")?;
        Ok(pp)
    }
}

fn write_data(ticks: &[NumberBytes]) -> Result<()> {
    let mut f = File::create(DATA_FILE)
        .map_err(|_| anyhow!("Failed to create file"))?;

    writeln!(f, "const DATA: &[ [u8; 8] ] = &[\n").with_context(|| format!("Failed to write ticks to file, {:?}", f))?;
    
    for record in ticks {
        writeln!(
            f,
            "    [{}],\n",
            record
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<String>>()
                .join(", ")
        ).with_context(|| format!("Failed to write ticks to file, {:?}", f))?;
    }
    writeln!(f, "];").with_context(|| format!("Failed to write ticks to file, {:?}", f))?;

    Ok(())
}

fn compile(memlimit:Option<usize>) -> Result<Nova<Local>>{
    println!("Compiling program {}...",PACKAGE_NAME);
    let mut opts = CompileOpts::new(PACKAGE_NAME);
    let memlimit = memlimit.unwrap_or(DEFAULT_MEMORY_LIMIT);
    opts.set_memlimit(memlimit); 
    let nova = nexus_sdk::nova::seq::Nova::compile(&opts)?;
    Ok(nova)
}

pub fn build(
    ticks: &[NumberBytes],
    memlimit:Option<usize>
) ->  Result<Nova<Local>> {
    // Define the output directory relative to the build script's location
    write_data(ticks)?;
    compile(memlimit)
}

pub fn execute_and_prove(prover:Nova<Local>, public_parameters:&PP,public_data:&PublicData) -> Result<Proof> {
    println!("Proving execution of vm...");
    let n_inv_sqrt = to_bytes(public_data.n_inv_sqrt);
    let n1_inv = to_bytes(public_data.n1_inv);
    let proof = prover.prove_with_input(public_parameters, &(n_inv_sqrt,n1_inv))?;
    Ok(proof)
}

pub fn execute(prover:Nova<Local>,public_data:&PublicData) -> Result<UncheckedView> {
    println!("Executing vm...");
    let n_inv_sqrt = to_bytes(public_data.n_inv_sqrt);
    let n1_inv = to_bytes(public_data.n1_inv);
    let view = prover.run_with_input(&(n_inv_sqrt,n1_inv))?;
    Ok(view)
}

pub fn verify(proof:&Proof, public_parameters:&PP) -> Result<()> {
    println!("Validating proof...");
    proof.verify(public_parameters).context("failed to verify proof")?;
    println!("  Succeeded!");
    Ok(())
}