use std::{io::BufRead, path::{Path, PathBuf}};

use anyhow::{bail, Context, Result};
use rand::thread_rng;
use rand_distr::{Distribution, Normal};
use serde::Deserialize;

pub enum TickSource {
    Random(usize),
    Jsonl(PathBuf),
    Csv(PathBuf),
}

impl TickSource {
    pub fn get_ticks(&self) -> Result<Vec<f32>> {
        match &self {
            TickSource::Random(size) => Ok(random_ticks(*size)),
            TickSource::Jsonl(file) => read_ticks_from_jsonl(file),
            TickSource::Csv(file) => read_ticks_from_csv(file)
        }
    }
}

/// Generates random ticks with a normal distribution
fn random_ticks(size:usize) -> Vec<f32> {

    println!("Generating random ticks");

    // Create a random number generator
    let mut rng = thread_rng();

    // Define the mean (mu) and standard deviation (sigma)
    let mu = 0.0f32;
    let sigma = 2.0f32.powf(24.0);

    // Create a Normal distribution with the specified mean and standard deviation
    let normal = Normal::new(mu, sigma).unwrap();
    (0..size).map(|_| normal.sample(&mut rng).round()).collect()
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Swap {
    evt_tx_hash: String,
    evt_index: u32,
    evt_block_time: String,
    evt_block_num: u64,
    sender: [u8; 20],
    recipient: [u8; 20],
    amount0: String,
    amount1: String,
    sqrt_price_x96: String,
    liquidity: String,
    pub tick: i64,
}

/// Reads ticks from a jsonl file containing uniswap Swap events
fn read_ticks_from_jsonl<P:AsRef<Path>>(file:P) -> Result<Vec<f32>> {
    let file = std::fs::File::open(file)
        .context("Failed to open jsonl file.")?;

    let reader = std::io::BufReader::new(file);

    let mut ticks = Vec::new();
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(reader);
    for result in rdr.deserialize() {
        let swap: Swap = result.context("Invalid swap format in jsonl")?;
        ticks.push(swap.tick as f32);
    }
    Ok(ticks)
}


/// Read ticks from a CSV file with a single column of numbers and a header
fn read_ticks_from_csv<P:AsRef<Path>>(file:P) -> Result<Vec<f32>> {
    let file = std::fs::File::open(file)
        .context("Failed to open csv file.")?;

    let mut reader = std::io::BufReader::new(file);

    let mut ticks = Vec::new();
    let mut line = String::new();
    // Skip the header line
    let _ = reader.read_line(&mut line).context("Failed to skip csv header line")?;
    line.clear();
    while reader.read_line(&mut line).context("Failed to read csv line")? > 0 
    {
        if let Ok(value) = line.trim().parse::<f32>() {
            ticks.push(value);
        } else {
            bail!("Invalid number in CSV");
        }
        line.clear();
    }
    Ok(ticks)
}


