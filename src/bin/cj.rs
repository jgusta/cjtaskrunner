use std::env;
use std::process;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.first().is_some_and(|arg| arg == "lsp") {
        if args.len() != 1 {
            eprintln!("cj: usage: cj lsp");
            process::exit(2);
        }
        cjtaskrunner::lsp::run_stdio().await;
        return;
    }

    match cjtaskrunner::run_cli(&args) {
        Ok(code) => process::exit(code),
        Err(err) => {
            eprintln!("cj: {err}");
            process::exit(2);
        }
    }
}
