use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    match cjtaskrunner::run_cli(&args) {
        Ok(code) => process::exit(code),
        Err(err) => {
            eprintln!("cj: {err}");
            process::exit(2);
        }
    }
}
