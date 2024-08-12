use clap::Parser;

mod common;
mod prover;
mod ticks;

use ticks::{PublicData, TickSource};
use prover::{build, execute, execute_and_prove, get_public_parameters, verify};


#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// A flag to specify ticks TickSource
    #[arg(short, long)]
    ticks: Option<String>,

    /// A flag to trigger watch mode
    #[arg(short, long)]
    watch: Option<String>,

    /// A flag to create proof
    #[arg(short, long)]
    proof: bool,

    /// A flag to verify proof
    #[arg(short, long)]
    verify: bool,

    /// zkVM Memory limit in MB
    #[arg(short, long)]
    memory:Option<usize>,

}



fn main() {
    let args = Args::parse();

    let pp = get_public_parameters().unwrap();

    match args.watch {

        // Continually read files from a dir.
        // When there are new files, load the ticks and generate a new proof using those ticks.
        // Start from the latest available block and load backwards until there are >= 8192 values for the proof.
        
        Some(_path) => {
            unimplemented!("Not implemented")
        }
        None => {
            let ticks_source = match args.ticks {
                Some(ticks) => TickSource::Csv(ticks),
                None => TickSource::Random,
            };

            let ticks = ticks_source.get_ticks().unwrap();

            let prover = build(&ticks, args.memory).unwrap();

            let public_data = PublicData::new(ticks);

            if !args.proof {
                let _ = execute(prover, &public_data).unwrap();
            }
            else {
                let proof = execute_and_prove(prover, &pp,&public_data).unwrap();
                  if args.verify {
                    verify(&proof, &pp).unwrap();
                }
            }
        }
    }
}







/* fn main2() {
    println!("Setting up Nova public parameters...");
    let pp: PP = PP::generate().expect("failed to generate parameters");

    let mut opts = CompileOpts::new(PACKAGE);
    opts.set_memlimit(8); // use an 8mb memory

    println!("Compiling guest program...");
    let prover: Nova<Local> = Nova::compile(&opts).expect("failed to compile guest program");

    println!("Proving execution of vm...");
    prover.prove_with_input(pp, input)


    let proof = prover.prove(&pp).expect("failed to prove program");

    println!(">>>>> Logging\n{}<<<<<", proof.logs().join(""));

    print!("Verifying execution...");
    proof.verify(&pp).expect("failed to verify proof");

    println!("  Succeeded!");
}
 */