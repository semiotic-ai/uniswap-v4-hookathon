use sp1_helper::build_program;
use std::path::Path;
use std::fs;
use std::fs::File;
use std::io::{self, Write, BufRead};

fn main() -> io::Result<()>{
        // Define the output directory relative to the build script's location
    let dest_path = Path::new("../program/src/data.rs");

    // Ensure the directory exists
    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut f = File::create(&dest_path)?;

    let file = std::fs::File::open("./src/ticks_8192.csv").expect("Could not open file");
    let mut reader = std::io::BufReader::new(file);
    let data = read_ticks_from_reader(&mut reader);
    // Generate the Rust code
    write!(f, "const DATA: &[ [u8; 4] ] = &[\n")?;
    for record in data {
        write!(f, "    [{}],\n", record.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(", "))?;
    }
    writeln!(f, "];")?;

    build_program("../program");
    Ok(())
}

fn read_ticks_from_reader<R: BufRead>(reader: &mut R) -> Vec<[u8; 4]> {
    let mut ticks = Vec::new();
    let mut line = String::new();
    // Skip the header line
    reader.read_line(&mut line).expect("Failed to read line");
    line.clear();
    while reader.read_line(&mut line).expect("Failed to read line") > 0 {
        if let Ok(value) = line.trim().parse::<i32>() {
            ticks.push(value.to_be_bytes());
        } else {
            panic!("Invalid number in CSV");
        }
        line.clear();
    }
    ticks
}
