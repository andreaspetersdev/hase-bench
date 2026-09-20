use std::process::ExitCode;

fn main() -> ExitCode {
    let invocation = match rust_rsync::parse_invocation(std::env::args_os().skip(1)) {
        Ok(invocation) => invocation,
        Err(error) => {
            eprintln!("rust-rsync: {error}");
            return ExitCode::from(error.exit_code());
        }
    };
    match rust_rsync::run(invocation) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("rust-rsync: {error}");
            ExitCode::from(error.exit_code())
        }
    }
}
