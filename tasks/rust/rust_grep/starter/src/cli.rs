use std::ffi::OsString;

/// Extend this option model to implement the command-line contract in TASK.md.
pub struct Options {
    pub arguments: Vec<OsString>,
}

pub fn parse(arguments: impl Iterator<Item = OsString>) -> Result<Options, String> {
    let arguments: Vec<_> = arguments.collect();
    if arguments.is_empty() {
        return Err("missing pattern".to_string());
    }
    Ok(Options { arguments })
}
