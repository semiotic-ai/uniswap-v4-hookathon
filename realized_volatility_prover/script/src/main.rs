//! A simple script to generate and verify the proof of a given program.
// use fixed::types::I15F17 as Fixed;

// use sp1_sdk::{ProverClient, SP1Stdin};

// const ELF: &[u8] = include_bytes!("../../program/elf/riscv32im-succinct-zkvm-elf");

use std::num::ParseIntError;

fn main() {
    // calculates and proves the volatility given the prices
    // TODO: this is sample data from the substream. Attach a pipeline to get it plainly
    let data: Vec<(&str, &str)> = vec![
        ("30000000000", "-11110957954678819042"),
        ("100000000000", "-37032707054197266894"),
        ("-133273119136", "49405342248031187577"),
        ("208492762943", "-77207953447434808545"),
        ("-1131012294", "419180762829823951"),
        ("672270300000", "-248778376767064561373"),
        ("1778631269", "-657843845874203202"),
        ("20000000000", "-7397064428025275384"),
        ("482086800000", "-178230515044344172669"),
        ("82315849716", "-30419095156401721403"),
        ("-1672770648", "618736755211914682"),
        ("217234590", "-80272093670403086"),
        ("2000000000", "-739034728308636029"),
        ("19332888765", "-7143717511682889290"),
        ("539871299110", "-199400221634678945504"),
        ("-1499173990", "554034582363680243"),
        ("10503764361", "-3877861637821964238"),
        ("5825000000", "-2150491152088852775"),
        ("-952568644149", "352288037037037060096"),
        ("30000000000", "-11091892065139417266"),
        ("-1271162294", "470446140858316382"),
        ("-13510020648", "5000000000000000000"),
    ];

    let ticks = [
        197314, 197313, 197315, 197311, 197311, 197301, 197301, 197300, 197293, 197291, 197291,
        197291, 197291, 197291, 197283, 197283, 197282, 197282, 197297, 197297, 197297, 197297,
    ];

    if ticks.len() != data.len() {
        panic!("invalid lengths of data and ticks")
    }

    let res = realized_volatility_calc(&data);
    println!("volatility with closing prices {:?}", res);

    let res2 = realized_volatility_calc2(&ticks);

    // let n = Fixed::from_num(swaps_amounts.len());

    // let mut stdin = SP1Stdin::new();
    // let n = 20u32;
    // stdin.write(&n);
    // let client = ProverClient::new();
    // let (pk, vk) = client.setup(ELF);
    // let mut proof = client.prove(&pk, stdin).expect("proving failed");

    // // Read output.
    // let a = proof.public_values.read::<u128>();
    // let b = proof.public_values.read::<u128>();
    // println!("a: {}", a);
    // println!("b: {}", b);

    // // Verify proof.
    // client.verify(&proof, &vk).expect("verification failed");

    // // Save proof.
    // proof
    //     .save("proof-with-io.json")
    //     .expect("saving proof failed");

    println!("successfully generated and verified proof for the program!")
}

// Calcualtes the realized volatility by getting the prices from the swap amounts
fn realized_volatility_calc(data: &Vec<(&str, &str)>) -> Result<f64, ParseIntError> {
    let mut closing_prices: Vec<f64> = Vec::new();

    for (amount0, amount1) in data {
        let num0 = amount0.parse::<i128>()?;
        let num1 = amount1.parse::<i128>()?;

        // Use absolute values for division
        let abs_num0 = num0.abs();
        let abs_num1 = num1.abs();

        if abs_num1 != 0 {
            let result = abs_num0 as f64 / abs_num1 as f64;
            closing_prices.push(result)
        } else {
            println!("Division by zero: {} / {}", abs_num0, abs_num1);
        }
    }

    if closing_prices.len() % 2 != 0 {
        panic!("The length of closing_prices must be even.");
    }

    let mut log_returns: Vec<f64> = Vec::new();

    // gets the log returns
    // L_r = (P_t / P_t-1)
    for i in (0..closing_prices.len()).step_by(2) {
        let price1 = closing_prices[i];
        let price2 = closing_prices[i + 1];

        let ratio = price2 as f64 / price1 as f64;
        log_returns.push(ratio.ln());
    }

    // Volatility calc

    //TODO: check if mean is necessary for the volatility
    //
    let mean_log_return = log_returns.iter().sum::<f64>() / log_returns.len() as f64;
    // equation for the realized volatility:
    // sqrt(sum(l_r ^2))
    let variance = log_returns
        .iter()
        .map(|&r| (r - mean_log_return).powi(2))
        .sum::<f64>()
        / (log_returns.len() as f64 - 1.0);
    println!("s2: {:?}", variance);

    let rv: f64 = variance.sqrt();
    Ok(rv * 100.0)
}

// calculates the volatility using the tick values
fn realized_volatility_calc2(ticks: &[i32]) -> Result<f64, ParseIntError> {
    //  u_i = tick_i - tick_i-1; // sequential return: log(price_i/price_i-1) = log(price_i) - log(price_i-1) = tick_i - tick_i-1
    let mut u_i: Vec<i32> = Vec::new();
    for i in (0..ticks.len()).step_by(2) {
        if i + 1 < ticks.len() {
            let diff = ticks[i + 1] - ticks[i];
            u_i.push(diff);
        }
    }

    println!("u_i: {:?}", u_i);
    // u_i2 = u_i ^ 2;
    let u_i2: Vec<i32> = u_i.iter().map(|&x| x.pow(2)).collect();

    // s2 = 1/(n-1) * ( sum( u_i2 ) - 1/n * sum( u_i )^2 )
    let n: f64 = ticks.len() as f64;
    if n > 1.0 {
        let sum_u_i2 = u_i2.iter().sum::<i32>() as f64;
        let sum_u_i = u_i.iter().sum::<i32>() as f64;
        let s2: f64 = (1.0 / (n - 1.0)) * ((sum_u_i2) - (1.0 / n) * (sum_u_i).powi(2));

        println!("s2: {}", s2);
    } else {
        println!("Not enough data points to calculate s2");
    }
    // let (sum_u, sum_u2) = ticks.iter().skip(1).fold((0.0, 0.0), |(su, su2), tick| {
    //     let ticks_curr = Fixed::from_be_bytes(*tick);
    //     let delta = ticks_curr - ticks_prev;
    //     ticks_prev = ticks_curr;
    //     (su + delta * n_inv_sqrt, su2 + delta * delta * n1_inv)
    // });
    // let s2 = sum_u2 - (sum_u * sum_u) * n1_inv;

    Ok(0.0)
}
