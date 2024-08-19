use clap::Parser;

mod volatility;
mod prover;
mod ticks;
mod watcher;

use ticks::TickSource;
use prover::{get_public_parameters, run};
use watcher::watch_directory;

const DEFAULT_SAMPLE_SIZE:usize = 8192;


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

    #[arg(short, long)]
    /// Number of ticks to sample
    sample:Option<usize>,
}



fn main() {
    let args = Args::parse();

    let pp = get_public_parameters().unwrap();

    match args.watch {

        // Continually read files from a dir.
        // When there are new files, load the ticks and generate a new proof using those ticks.
        // Start from the latest available block and load backwards until there are >= 8192 values for the proof.
        
        Some(path) => {
            let mut latest_block = 0;
            loop {
                match watch_directory(&pp, &path, latest_block, args.memory,args.proof,args.verify) {
                    Ok(block) => {
                        latest_block = block;
                        println!("Latest block: {}", block);
                    }
                    Err(error) => println!("Error loading and proving {}", error),
                }
            }
        }
        None => {
            let ticks_source = match args.ticks {
                Some(ticks) => TickSource::Csv(ticks.into()),
                None => TickSource::Random(args.sample.unwrap_or(DEFAULT_SAMPLE_SIZE)),
            };

            let ticks = ticks_source.get_ticks().unwrap();

            run(&pp,&ticks,args.memory,args.proof,args.verify).unwrap();
        }
    }
}
