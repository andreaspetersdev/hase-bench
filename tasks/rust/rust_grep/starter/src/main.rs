mod cli;
mod matcher;
mod output;
mod stream;
mod walk;

fn main() {
    let options = match cli::parse(std::env::args_os().skip(1)) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("rust_grep: {message}");
            std::process::exit(2);
        }
    };
    std::process::exit(output::run(options));
}
