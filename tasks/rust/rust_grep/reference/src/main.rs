mod cli;
mod matcher;
mod runner;
mod walk;

use std::io::{self, Write};

fn main() {
    let result = cli::parse(std::env::args_os().skip(1)).and_then(runner::run);
    let status = match result {
        Ok(status) => status,
        Err(message) => {
            let _ = writeln!(io::stderr().lock(), "rust_grep: {message}");
            2
        }
    };
    std::process::exit(status);
}
