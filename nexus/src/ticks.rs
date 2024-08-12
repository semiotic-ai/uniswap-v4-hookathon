use std::{io::BufRead, path::Path};

use anyhow::{bail, Context, Result};
use crate::common::*;
use rand::thread_rng;
use rand_distr::{Distribution, Normal};
use serde::Deserialize;

const N: usize = 8192;

pub enum TickSource {
    Random,
    Jsonl(String),
    Csv(String),
}

impl TickSource {
    pub fn get_ticks(&self) -> Result<Vec<NumberBytes>> {
        match &self {
            TickSource::Random => Ok(random_ticks()),
            TickSource::Jsonl(file) => read_ticks_from_jsonl(file),
            TickSource::Csv(file) => read_ticks_from_csv(file)
        }
    }
}

/// Generates random ticks with a normal distribution
fn random_ticks() -> Vec<NumberBytes> {

    println!("Generating random ticks");

    // Create a random number generator
    let mut rng = thread_rng();

    // Define the mean (mu) and standard deviation (sigma)
    let mu = 0.0;
    let sigma = 2.0f32.powf(24.0);

    // Create a Normal distribution with the specified mean and standard deviation
    let normal = Normal::new(mu, sigma).unwrap();
    let rand_vec: Vec<i64> = (0..N)
        .map(|_| {
            let r_f64: f64 = normal.sample(&mut rng).into();
            r_f64.round() as i64
        })
        .collect();
    rand_vec.iter().map(|x| x.to_be_bytes()).collect()
}

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
fn read_ticks_from_jsonl<P:AsRef<Path>>(file:P) -> Result<Vec<NumberBytes>> {
    let file = std::fs::File::open(file)
        .context("Failed to open jsonl file.")?;

    let reader = std::io::BufReader::new(file);

    let mut ticks = Vec::new();
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(reader);
    for result in rdr.deserialize() {
        let swap: Swap = result.context("Invalid swap format in jsonl")?;
        ticks.push((swap.tick as i64).to_be_bytes());
    }
    Ok(ticks)
}


/// Read ticks from a CSV file with a single column of numbers and a header
fn read_ticks_from_csv<P:AsRef<Path>>(file:P) -> Result<Vec<NumberBytes>> {
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
        if let Ok(value) = line.trim().parse::<i64>() {
            ticks.push((value).to_be_bytes());
        } else {
            bail!("Invalid number in CSV");
        }
        line.clear();
    }
    Ok(ticks)
}

#[derive(Clone)]
pub struct PublicData {
    pub n_inv_sqrt: Fixed,
    pub n1_inv: Fixed,
    pub s2: Fixed,
}

impl PublicData {

    pub fn new(ticks: Vec<NumberBytes>) -> Self {
        let n = Fixed::from_num(ticks.len());

        let n_inv_sqrt = Fixed::ONE / n.sqrt();
        let n1_inv = Fixed::ONE / n;
    
        let data = ticks.into_iter().map(|x| to_fixed(x)).collect::<Vec<Fixed>>();
    
        let s2 = tick_volatility(&data, n_inv_sqrt, n1_inv);
        
        println!("Volatility squared {}", s2);

        PublicData {
            n_inv_sqrt,
            n1_inv,
            s2,
        }
    }
}
