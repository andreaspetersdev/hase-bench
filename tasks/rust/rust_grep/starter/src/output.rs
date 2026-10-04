use crate::cli::Options;

/// TODO: coordinate input scanning, selection, presentation, and exit status.
pub fn run(options: Options) -> i32 {
    let _ = options.arguments;
    let _ = crate::matcher::matches(b"", b"");
    let _ = crate::stream::scan(b"");
    let _ = crate::walk::inputs(&[]);
    1
}
