//! A simple script to generate and verify the proof of a given program.

mod build_elf;
mod prove;
mod watcher;

use build_elf::{read_ticks, TickSource};
use clap::Parser;

const ELF_PATH: &str = "../program/elf/riscv32im-succinct-zkvm-elf";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// A flag to specify ticks TickSource
    #[arg(short, long)]
    ticks: Option<String>,

    /// A flag to trigger watch mode
    #[arg(short, long)]
    watch: Option<String>,

    /// A flag to execute only, no proof generation
    #[arg(short, long)]
    execute: bool,
}

fn main() {
    let args = Args::parse();
    match args.watch {
        // Continually read files from a dir.
        // When there are new files, load the ticks and generate a new proof using those ticks.
        // Start from the latest available block and load backwards until there are >= 8192 values for the proof.
        Some(path) => {
            let mut latest_block = 0;
            loop {
                match watcher::watch_directory(ELF_PATH, &path, latest_block, args.execute) {
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
                Some(ticks) => TickSource::Jsonl(ticks),
                None => TickSource::Random,
            };
            let ticks = read_ticks(ticks_source);
            let (elf, stdin, client) = prove::setup(ELF_PATH, ticks).unwrap();
            if args.execute {
                prove::exec(elf.as_slice(), stdin, client).unwrap();
            } else {
                prove::prove(elf.as_slice(), stdin, client).unwrap();
            }
        }
    }
}
