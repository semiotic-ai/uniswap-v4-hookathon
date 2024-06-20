use crate::build_elf::{read_ticks_from_jsonl, NumberBytes};
use crate::prove;
use anyhow::Result;
use regex::Regex;
use std::cmp::Reverse;
use std::fs;
use std::path::PathBuf;

// Given a the path to a directory:
// Loop and check if there are any new files. If so, start from the latest file, read all indices
// in the file, and store in vector of ticks. If there are less than 8192 entries in the vector,
// read the next latest file and continue.
pub fn watch_directory(
    elf_path: &str,
    path: &str,
    latest_block: u64,
    exec_flag: bool,
) -> Result<u64> {
    let (ticks, latest_block) = match read_latest_ticks(path, latest_block) {
        Ok(ticks) => ticks,
        Err(error) => return Err(error),
    };
    let (elf, stdin, client) = prove::setup(elf_path, ticks)?;
    if exec_flag {
        prove::exec(elf.as_slice(), stdin, client)?;
    } else {
        prove::prove(elf.as_slice(), stdin, client)?;
    }

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

fn read_latest_ticks(directory: &str, latest_block: u64) -> Result<(Vec<NumberBytes>, u64)> {
    let mut latest_file = String::new();

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
    let mut ticks: Vec<NumberBytes> = Vec::new();
    for file in files {
        let file = std::fs::File::open(file).expect("Could not open file");
        let mut reader = std::io::BufReader::new(file);
        let new_ticks = read_ticks_from_jsonl(&mut reader)?;
        ticks.extend(new_ticks.into_iter());
        if ticks.len() >= 8192 {
            break;
        };
    }
    Ok((ticks, new_latest_block))
}
