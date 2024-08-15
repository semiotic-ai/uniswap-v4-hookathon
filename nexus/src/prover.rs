
use anyhow::{Result, anyhow, Context};
use nexus_sdk::compile::CompileOpts;
use nexus_sdk::nova::seq::*;
use nexus_sdk::*;
use views::UncheckedView;

use std::time::Instant;
use std::{fs::File, path::Path};
use std::io::Write;

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

fn write_data(ticks: &[f32]) -> Result<()> {
    let mut f = File::create(DATA_FILE)
        .map_err(|_| anyhow!("Failed to create file"))?;

    writeln!(f, "const DATA: &[ f32 ] = &[\n").with_context(|| format!("Failed to write ticks to file, {:?}", f))?;
    
    for record in ticks {
        writeln!(f,"    {:.1}f32,\n",record).with_context(|| format!("Failed to write ticks to file, {:?}", f))?;
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

fn build(
    ticks: &[f32],
    memlimit:Option<usize>
) ->  Result<Nova<Local>> {
    // Define the output directory relative to the build script's location
    write_data(ticks)?;
    compile(memlimit)
}

fn execute_and_prove(prover:Nova<Local>, public_parameters:&PP) -> Result<Proof> {
    println!("Proving execution of vm...");
    let proof = prover.prove(public_parameters)?;
    Ok(proof)
}

fn execute(prover:Nova<Local>) -> Result<UncheckedView> {
    println!("Executing vm...");
    let view = prover.run()?;
    Ok(view)
}

fn verify_proof(proof:&Proof, public_parameters:&PP) -> Result<()> {
    println!("Validating proof...");
    proof.verify(public_parameters).context("failed to verify proof")?;
    println!("  Succeeded!");
    Ok(())
}


pub fn run(pp:&PP,ticks:&[f32],memlimit:Option<usize>,proof:bool,verify:bool) -> Result<()> {

    let now = Instant::now();

    let prover = build(ticks, memlimit)?;

    println!("Prover built in {}sec.", now.elapsed().as_secs());

    //let vol = Volatility::new(&ticks);

    if !proof {
        let now = Instant::now();
        let _ = execute(prover).unwrap();
        println!("Execution completed in {}sec.", now.elapsed().as_secs());
    }
    else {
        let now = Instant::now();
        let proof = execute_and_prove(prover, &pp).unwrap();
        println!("Execution and proof generated in {}sec.", now.elapsed().as_secs());
          if verify {
            let now = Instant::now();
            verify_proof(&proof, &pp).unwrap();
            println!("Proof verified in {}sec.", now.elapsed().as_secs());
        }
    }
    Ok(())
}
