#![cfg_attr(target_arch = "riscv32", no_std, no_main)]

use nexus_rt::write_output;

include!("data.rs"); // Include the data module
include!("../../volatility.rs"); // Include the types module

#[nexus_rt::main]
pub fn main() {
    
    let v = Volatility::new(DATA);

    write_output(&v);
    
}