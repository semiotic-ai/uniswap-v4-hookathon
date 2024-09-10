use blitzar::{compute::init_backend, proof::InnerProductProof};
use proof_of_sql::{
    base::database::{owned_table_utility::*, OwnedTableTestAccessor, TestAccessor},
    sql::{parse::QueryExpr, proof::QueryProof},
};
use std::{
    env::args,
    fs::File,
    io::{stdout, BufRead, BufReader, Write},
    iter,
    time::Instant,
};

const FILE: &str = "ticks_8192.csv";

fn start_timer(message: &str) -> Instant {
    print!("{}...", message);
    stdout().flush().unwrap();
    Instant::now()
}
fn end_timer(instant: Instant) {
    println!(" {:?}", instant.elapsed());
}

fn main() {
    let querystr = args().nth(1).expect("No arguments");

    let ticks = File::open(FILE)
        .map(|file| BufReader::new(file))
        .map(|reader| reader.lines())
        .expect("Ticks file can not be read")
        .skip(1)
        .map(|line| {
            line.map(|value| str::parse::<i64>(&value).expect("Can not parse value"))
                .expect("Can not read line")
        })
        .collect::<Vec<_>>();

    let timer = start_timer("Warming up GPU");
    init_backend();
    end_timer(timer);
    let timer = start_timer("Loading data");

    let mut accessor = OwnedTableTestAccessor::<InnerProductProof>::new_empty_with_setup(());
    accessor.add_table(
        "sxt.table".parse().unwrap(),
        owned_table([
            varchar("pool", iter::repeat("usdc-weth").take(8192)),
            bigint("ticks", ticks),
        ]),
        0,
    );
    end_timer(timer);
    let timer = start_timer("Parsing Query");

    let mut query =
        QueryExpr::try_new(querystr.parse().unwrap(), "sxt".parse().unwrap(), &accessor).unwrap();
    end_timer(timer);
    let timer = start_timer("Generating Proof");

    let (proof, serialized_result) =
        QueryProof::<InnerProductProof>::new(query.proof_expr(), &accessor, &());
    end_timer(timer);
    let timer = start_timer("Verifying Proof");

    let result = proof.verify(query.proof_expr(), &accessor, &serialized_result, &());
    end_timer(timer);
    match result {
        Ok(result) => {
            println!("Valid proof!");
            println!("Query: {}", querystr);
            println!("Query result: {:?}", result.table);
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
}
