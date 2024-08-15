use crate::prover::run;
use crate::ticks::TickSource;
use anyhow::Result;
use regex::Regex;
use std::cmp::Reverse;
use std::fs;
use std::path::PathBuf;
use nexus_sdk::nova::seq::PP;

// Given a the path to a directory:
// Loop and check if there are any new files. If so, start from the latest file, read all indices
// in the file, and store in vector of ticks. If there are less than 8192 entries in the vector,
// read the next latest file and continue.
pub fn watch_directory(
    public_params:&PP,
    path: &str,
    latest_block: u64,
    memlimit: Option<usize>,
    proof:bool,
    verify:bool,
) -> Result<u64> {

    let (ticks, latest_block) = match read_latest_ticks(path, latest_block) {
        Ok(ticks) => ticks,
        Err(error) => return Err(error),
    };

    run(public_params, &ticks, memlimit, proof, verify)?;

    Ok(latest_block)
}

// A function to parse the .jsonl files output by the realized_volatility_substream.
// Returns start and end block numbers for entries in the file.
fn parse_filename(filename: &str) -> Result<(u64, u64)> {
    let re = Regex::new(r"(\d+)-(\d+)\.jsonl")?;

    if let Some(caps) = re.captures(filename) {
        let start_block: u64 = caps.get(1).unwrap().as_str().parse()?;
        let end_block: u64 = caps.get(2).unwrap().as_str().parse()?;
        Ok((start_block, end_block))
    } else {
        Err(anyhow::anyhow!(
            "Filename does not match the expected format."
        ))
    }
}

fn read_latest_ticks(directory: &str, latest_block: u64) -> Result<(Vec<f32>, u64)> {
    let mut files: Vec<PathBuf> = fs::read_dir(directory)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect();

    files.sort_by_key(|name| {
        let (_, end_block) = parse_filename(name.to_str().expect("bad file name")).unwrap();
        Reverse(end_block)
    });
    let (_, new_latest_block) = parse_filename(files[0].to_str().expect("bad file name"))?;
    if new_latest_block <= latest_block {
        return Err(anyhow::anyhow!("No new blocks"));
    }
    println!("Latest block: {}", new_latest_block);
    let mut ticks: Vec<f32> = Vec::new();
    for file in files {
        let (start_block, _) = parse_filename(file.to_str().expect("bad file name"))?;

        let ticksource = TickSource::Jsonl(file);
        let new_ticks = ticksource.get_ticks()?;
        ticks.extend(new_ticks.into_iter());
        let num_blocks = new_latest_block - start_block;
        if num_blocks >= 8192 {
            break;
        };
    }
    Ok((ticks, new_latest_block))
}
