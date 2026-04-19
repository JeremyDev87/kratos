use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    process::exit(kratos_cli::run_cli(&args));
}
