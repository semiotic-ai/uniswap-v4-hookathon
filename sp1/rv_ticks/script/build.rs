use sp1_helper::build_program;
use std::path::Path;
use std::fs;
use std::fs::File;
use std::io::{self, Write, BufRead, Read};
use std::env;
use rand_distr::{Distribution, Normal};
use rand::thread_rng;
use serde::Deserialize;
use csv;
use std::error::Error;

type NumberBytes = [u8; 8];
const N: usize = 8192;

fn main() -> io::Result<()>{
        // Define the output directory relative to the build script's location
    let dest_path = Path::new("../program/src/data.rs");
    let mut f = File::create(dest_path)?;
    let data: Vec<NumberBytes>;

    if env::var("RANDOM_TICKS").is_ok() {
        data = ticks();
    } else if env::var("STDIN_JSONL").is_ok() {
        let file = std::fs::File::open("./src/exemplar.jsonl").expect("Could not open file");
        let mut reader = std::io::BufReader::new(file);
        data = read_ticks_from_jsonl(&mut reader).unwrap();
        println!("{:?}", data);
    } else {
        let file = std::fs::File::open("./src/ticks_8192.csv").expect("Could not open file");
        let mut reader = std::io::BufReader::new(file);
        data = read_ticks_from_reader(&mut reader);
    }
    // Ensure the directory exists
    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Generate the Rust code
    write!(f, "const DATA: &[ [u8; 8] ] = &[\n")?;
    for record in data {
        write!(f, "    [{}],\n", record.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(", "))?;
    }
    writeln!(f, "];")?;

    build_program("../program");
    Ok(())
}

#[derive(Debug, Deserialize)]
struct Swap {
    evt_tx_hash: String,
    evt_index: u32,
    evt_block_time: String,
    evt_block_num: u64,
    sender: [u8; 20] ,
    recipient: [u8; 20],
    amount0: String,
    amount1: String,
    sqrt_price_x96: String,
    liquidity: String,
    tick: i64 
}

fn read_ticks_from_jsonl<R: Read>(reader: &mut R) -> Result<Vec<NumberBytes>, Box<dyn Error>> {
    let mut ticks = Vec::new();
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(reader);
    for result in rdr.deserialize() {
        let swap: Swap = result?;
        ticks.push((swap.tick as i64).to_be_bytes());
    }
    println!("{:?}", ticks);
    Ok(ticks)
}
fn read_ticks_from_reader<R: BufRead>(reader: &mut R) -> Vec<NumberBytes> {
    let mut ticks = Vec::new();
    let mut line = String::new();
    // Skip the header line
    reader.read_line(&mut line).expect("Failed to read line");
    line.clear();
    while reader.read_line(&mut line).expect("Failed to read line") > 0 {
        if let Ok(value) = line.trim().parse::<i64>() {
            ticks.push((value as i64).to_be_bytes());
        } else {
            panic!("Invalid number in CSV");
        }
        line.clear();
    }
    ticks
}

fn ticks() -> Vec<NumberBytes> {
    // Create a random number generator
    let mut rng = thread_rng();
    
    // Define the mean (mu) and standard deviation (sigma)
    let mu = 0.0;
    let sigma = 2.0f32.powf(24.0);

    // Create a Normal distribution with the specified mean and standard deviation
    let normal = Normal::new(mu, sigma).unwrap();
    let rand_vec: Vec<i64> = (0..N).map(|_| {
        let r_f64: f64 = normal.sample(&mut rng).into();
        r_f64.round() as i64 }).collect();
    rand_vec.iter().map(|x| x.to_be_bytes()).collect()
}
