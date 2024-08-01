use anyhow::{Result, Context};
use chrono::Local;
use rand::thread_rng;
use rand_distr::{Distribution, Normal};
use serde::Deserialize;
use std::fs::File;
use std::io::{BufRead, Read, Write};
use std::{
    io::BufReader,
    process::{Command, Stdio},
    thread,
};
use jsonl::read;

pub type NumberBytes = [u8; 8];

const N: usize = 8192;

pub enum TickSource {
    Random,
    Jsonl(String),
    Csv(String),
}

#[derive(Debug, Deserialize)]
pub struct Swap {
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

pub fn read_ticks(source: TickSource) -> Vec<NumberBytes> {
    match source {
        TickSource::Random => ticks(),
        TickSource::Jsonl(file) => {
            let file = std::fs::File::open(file).expect("Could not open file");
            let mut reader = std::io::BufReader::new(file);
            read_ticks_from_jsonl(&mut reader).unwrap()
        }
        TickSource::Csv(file) => {
            let file = std::fs::File::open(file).expect("Could not open file");
            let mut reader = std::io::BufReader::new(file);
            read_ticks_from_reader(&mut reader)
        }
    }
}

fn write_ticks_to_file(ticks: Vec<NumberBytes>, file: &str) -> Result<()> {
    let mut f = File::create(file)?;

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

pub fn build_elf(
    ticks: Vec<NumberBytes>,
    tick_dest_file: &str,
    program_path: &str,
) -> Result<()> {
    // Define the output directory relative to the build script's location
    write_ticks_to_file(ticks, tick_dest_file)?;
    build_program(program_path);

    Ok(())
}

pub fn read_ticks_from_jsonl<R: BufRead>(reader: &mut R) -> Result<Vec<NumberBytes>> {
    let mut ticks = Vec::new();
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(reader);
    for result in rdr.deserialize() {
        let swap: Swap = result?;
        ticks.push((swap.tick as i64).to_be_bytes());
    }
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
            ticks.push((value).to_be_bytes());
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
    let rand_vec: Vec<i64> = (0..N)
        .map(|_| {
            let r_f64: f64 = normal.sample(&mut rng).into();
            r_f64.round() as i64
        })
        .collect();
    rand_vec.iter().map(|x| x.to_be_bytes()).collect()
}

fn current_datetime() -> String {
    let now = Local::now();
    now.format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn build_program(path: &str) {
    println!("path: {:?}", path);
    let program_dir = std::path::Path::new(path);

    // Tell cargo to rerun the script only if program/{src, Cargo.toml, Cargo.lock} changes
    // Ref: https://doc.rust-lang.org/nightly/cargo/reference/build-scripts.html#rerun-if-changed
    let dirs = vec![
        program_dir.join("src"),
        program_dir.join("Cargo.toml"),
        program_dir.join("Cargo.lock"),
    ];
    for dir in dirs {
        println!("cargo::rerun-if-changed={}", dir.display());
    }

    // Print a message so the user knows that their program was built. Cargo caches warnings emitted
    // from build scripts, so we'll print the date/time when the program was built.
    let metadata_file = program_dir.join("Cargo.toml");
    let mut metadata_cmd = cargo_metadata::MetadataCommand::new();
    let metadata = metadata_cmd.manifest_path(metadata_file).exec().unwrap();
    let root_package = metadata.root_package();
    let root_package_name = root_package
        .as_ref()
        .map(|p| p.name.as_str())
        .unwrap_or("Program");
    println!(
        "cargo:warning={} built at {}",
        root_package_name,
        current_datetime()
    );

    let status = execute_build_cmd(&program_dir)
        .unwrap_or_else(|_| panic!("Failed to build `{}`.", root_package_name));
    if !status.success() {
        panic!("Failed to build `{}`.", root_package_name);
    }
}

/// Executes the `cargo prove build` command in the program directory
fn execute_build_cmd(
    program_dir: &impl AsRef<std::path::Path>,
) -> Result<std::process::ExitStatus, std::io::Error> {
    // Check if RUSTC_WORKSPACE_WRAPPER is set to clippy-driver (i.e. if `cargo clippy` is the current
    // compiler). If so, don't execute `cargo prove build` because it breaks rust-analyzer's `cargo clippy` feature.
    let is_clippy_driver = std::env::var("RUSTC_WORKSPACE_WRAPPER")
        .map(|val| val.contains("clippy-driver"))
        .unwrap_or(false);
    if is_clippy_driver {
        println!("cargo:warning=Skipping build due to clippy invocation.");
        return Ok(std::process::ExitStatus::default());
    }

    let mut cmd = Command::new("cargo");
    cmd.current_dir(program_dir)
        .args(["prove", "build"])
        .env_remove("RUSTC")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn()?;

    let stdout = BufReader::new(child.stdout.take().unwrap());
    let stderr = BufReader::new(child.stderr.take().unwrap());

    // Pipe stdout and stderr to the parent process with [sp1] prefix
    let stdout_handle = thread::spawn(move || {
        stdout.lines().for_each(|line| {
            println!("[sp1] {}", line.unwrap());
        });
    });
    stderr.lines().for_each(|line| {
        eprintln!("[sp1] {}", line.unwrap());
    });

    stdout_handle.join().unwrap();

    child.wait()
}
