use crate::cli::{Mode, Options};
use crate::matcher::Matcher;
use crate::walk::{self, Input};
use std::collections::VecDeque;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;

const MAX_LINE: usize = 16 * 1024 * 1024;

#[derive(Debug)]
enum ScanFailure {
    Input(String),
    Output(io::Error, bool),
}

pub fn run(options: Options) -> Result<i32, String> {
    let matcher = Matcher::new(&options)?;
    let inputs = walk::collect(&options)?;
    let show_name = options
        .force_filename
        .unwrap_or(options.recursive || options.paths.len() > 1);
    let mode = options.mode();
    let mut selected = false;
    let mut had_error = false;
    let mut output = io::stdout().lock();
    for input in inputs {
        let path = match input {
            Input::File(path) => path,
            Input::Error(error) => {
                eprintln!("rust_grep: {error}");
                had_error = true;
                continue;
            }
        };
        let name = if path == Path::new("-") {
            "(standard input)".to_string()
        } else {
            path.display().to_string()
        };
        if !options.fixed && path != Path::new("-") && path.to_str().is_none() {
            eprintln!("rust_grep: {name}: regex path is not UTF-8");
            had_error = true;
            continue;
        }
        let output_name = output_name(&path);
        let result = if path == Path::new("-") {
            scan(
                io::stdin().lock(),
                &output_name,
                show_name,
                &options,
                &matcher,
                &mut output,
            )
        } else {
            File::open(&path)
                .map_err(|error| ScanFailure::Input(error.to_string()))
                .and_then(|file| {
                    scan(
                        BufReader::new(file),
                        &output_name,
                        show_name,
                        &options,
                        &matcher,
                        &mut output,
                    )
                })
        };
        match result {
            Ok(found) => {
                selected |= found;
                if found && mode == Mode::Quiet {
                    return Ok(if had_error { 2 } else { 0 });
                }
            }
            Err(ScanFailure::Input(error)) => {
                eprintln!("rust_grep: {}: {error}", path.display());
                had_error = true;
            }
            Err(ScanFailure::Output(error, output_selected)) => {
                if error.kind() == io::ErrorKind::BrokenPipe {
                    return Ok(if (selected || output_selected) && !had_error {
                        0
                    } else {
                        2
                    });
                }
                eprintln!("rust_grep: output: {error}");
                return Ok(2);
            }
        }
    }
    Ok(if !had_error {
        if selected { 0 } else { 1 }
    } else {
        2
    })
}

fn output_name(path: &Path) -> Vec<u8> {
    if path == Path::new("-") {
        return b"(standard input)".to_vec();
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        path.as_os_str().as_bytes().to_vec()
    }
    #[cfg(windows)]
    {
        path.display().to_string().into_bytes()
    }
}

fn scan<R: BufRead, W: Write>(
    mut reader: R,
    name: &[u8],
    show_name: bool,
    options: &Options,
    matcher: &Matcher,
    output: &mut W,
) -> Result<bool, ScanFailure> {
    let mode = options.mode();
    let grouped = mode == Mode::Normal && (options.before > 0 || options.after > 0);
    let mut matched = false;
    let mut count = 0usize;
    let mut binary = false;
    let mut binary_reported = false;
    let mut line = Vec::new();
    let mut line_no = 0usize;
    let mut before = VecDeque::<(usize, Vec<u8>)>::new();
    let mut after_remaining = 0;
    let mut last_output = None;
    loop {
        line.clear();
        if !read_line_bounded(&mut reader, &mut line).map_err(ScanFailure::Input)? {
            break;
        }
        line_no += 1;
        if line.last() == Some(&b'\n') {
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
        }
        if line.len() > MAX_LINE {
            return Err(ScanFailure::Input(
                "logical line exceeds 16 MiB".to_string(),
            ));
        }
        if !options.fixed && std::str::from_utf8(&line).is_err() {
            return Err(ScanFailure::Input("regex input is not UTF-8".to_string()));
        }
        binary |= line.contains(&0);
        let hit = matcher.matches(&line);
        let selected_line = hit != options.invert;
        if selected_line {
            matched = true;
            count += 1;
            match mode {
                Mode::Quiet | Mode::FilesWith | Mode::FilesWithout => break,
                Mode::Count => {}
                Mode::OnlyMatching | Mode::Normal if binary => {
                    if !binary_reported {
                        output
                            .write_all(b"Binary file ")
                            .and_then(|_| output.write_all(name))
                            .and_then(|_| output.write_all(b" matches\n"))
                            .map_err(|error| ScanFailure::Output(error, true))?;
                        binary_reported = true;
                    }
                }
                Mode::OnlyMatching => {
                    if !options.invert {
                        for (start, end) in matcher.spans(&line) {
                            write_prefix(output, name, line_no, show_name, options.number, true)
                                .map_err(|error| ScanFailure::Output(error, true))?;
                            output
                                .write_all(&line[start..end])
                                .and_then(|_| output.write_all(b"\n"))
                                .map_err(|error| ScanFailure::Output(error, true))?;
                        }
                    }
                }
                Mode::Normal => {
                    for (number, content) in &before {
                        emit_record(
                            output,
                            name,
                            *number,
                            content,
                            false,
                            show_name,
                            options.number,
                            grouped,
                            &mut last_output,
                        )
                        .map_err(|error| ScanFailure::Output(error, true))?;
                    }
                    emit_record(
                        output,
                        name,
                        line_no,
                        &line,
                        true,
                        show_name,
                        options.number,
                        grouped,
                        &mut last_output,
                    )
                    .map_err(|error| ScanFailure::Output(error, true))?;
                    after_remaining = options.after;
                }
            }
        } else if mode == Mode::Normal && after_remaining > 0 {
            if !binary {
                emit_record(
                    output,
                    name,
                    line_no,
                    &line,
                    false,
                    show_name,
                    options.number,
                    grouped,
                    &mut last_output,
                )
                .map_err(|error| ScanFailure::Output(error, matched))?;
            }
            after_remaining -= 1;
        }
        if grouped && options.before > 0 {
            before.push_back((line_no, line.clone()));
            while before.len() > options.before {
                before.pop_front();
            }
        }
    }
    match mode {
        Mode::FilesWith if matched => output
            .write_all(name)
            .and_then(|_| output.write_all(b"\n"))
            .map_err(|error| ScanFailure::Output(error, true))?,
        Mode::FilesWithout if !matched => output
            .write_all(name)
            .and_then(|_| output.write_all(b"\n"))
            .map_err(|error| ScanFailure::Output(error, true))?,
        Mode::Count => {
            if show_name {
                output
                    .write_all(name)
                    .and_then(|_| output.write_all(b":"))
                    .map_err(|error| ScanFailure::Output(error, matched))?;
            }
            writeln!(output, "{count}").map_err(|error| ScanFailure::Output(error, matched))?;
        }
        _ => {}
    }
    Ok(if mode == Mode::FilesWithout {
        !matched
    } else {
        matched
    })
}

fn write_prefix<W: Write>(
    output: &mut W,
    name: &[u8],
    line_no: usize,
    show_name: bool,
    number: bool,
    selected: bool,
) -> io::Result<()> {
    let delimiter = if selected { ':' } else { '-' };
    if show_name {
        output
            .write_all(name)
            .and_then(|_| output.write_all(&[delimiter as u8]))?;
    }
    if number {
        write!(output, "{line_no}{delimiter}")?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn emit_record<W: Write>(
    output: &mut W,
    name: &[u8],
    line_no: usize,
    line: &[u8],
    selected: bool,
    show_name: bool,
    number: bool,
    grouped: bool,
    last_output: &mut Option<usize>,
) -> io::Result<()> {
    if last_output.is_some_and(|previous| line_no <= previous) {
        return Ok(());
    }
    if grouped && last_output.is_some_and(|previous| line_no > previous + 1) {
        output.write_all(b"--\n")?;
    }
    write_prefix(output, name, line_no, show_name, number, selected)?;
    output
        .write_all(line)
        .and_then(|_| output.write_all(b"\n"))?;
    *last_output = Some(line_no);
    Ok(())
}

fn read_line_bounded<R: BufRead>(reader: &mut R, line: &mut Vec<u8>) -> Result<bool, String> {
    loop {
        let available = reader.fill_buf().map_err(|error| error.to_string())?;
        if available.is_empty() {
            return Ok(!line.is_empty());
        }
        let take = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(available.len(), |index| index + 1);
        let ended = available[take - 1] == b'\n';
        if line.len() + take > MAX_LINE + 2 {
            return Err("logical line exceeds 16 MiB".to_string());
        }
        line.extend_from_slice(&available[..take]);
        reader.consume(take);
        if ended {
            return Ok(true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ScanFailure, read_line_bounded, scan};
    use crate::cli::Options;
    use crate::matcher::Matcher;
    use std::io::{self, BufReader, Cursor, Read, Write};

    #[test]
    fn one_byte_buffers_preserve_crlf_and_final_record() {
        let source = Cursor::new(b"first\r\nsecond\nlast".as_slice());
        let mut reader = BufReader::with_capacity(1, source);
        let mut line = Vec::new();
        for expected in [b"first\r\n".as_slice(), b"second\n", b"last"] {
            line.clear();
            assert!(read_line_bounded(&mut reader, &mut line).unwrap());
            assert_eq!(line, expected);
        }
        line.clear();
        assert!(!read_line_bounded(&mut reader, &mut line).unwrap());
    }

    struct FailAfterLine(bool);

    impl Read for FailAfterLine {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            if self.0 {
                return Err(io::Error::other("injected read failure"));
            }
            self.0 = true;
            buffer[..4].copy_from_slice(b"hit\n");
            Ok(4)
        }
    }

    struct BrokenOutput;

    impl Write for BrokenOutput {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::from(io::ErrorKind::BrokenPipe))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn fixed_options() -> Options {
        Options {
            patterns: vec![b"hit".to_vec()],
            fixed: true,
            ..Options::default()
        }
    }

    #[test]
    fn read_error_keeps_earlier_selected_output() {
        let options = fixed_options();
        let matcher = Matcher::new(&options).unwrap();
        let reader = BufReader::with_capacity(4, FailAfterLine(false));
        let mut output = Vec::new();
        let result = scan(reader, b"stdin", false, &options, &matcher, &mut output);
        assert!(
            matches!(result, Err(ScanFailure::Input(message)) if message.contains("injected read failure"))
        );
        assert_eq!(output, b"hit\n");
    }

    #[test]
    fn broken_output_remembers_whether_a_result_was_selected() {
        let options = fixed_options();
        let matcher = Matcher::new(&options).unwrap();
        let result = scan(
            Cursor::new(b"hit\n"),
            b"stdin",
            false,
            &options,
            &matcher,
            &mut BrokenOutput,
        );
        assert!(matches!(result, Err(ScanFailure::Output(error, true))
                         if error.kind() == io::ErrorKind::BrokenPipe));

        let count_options = Options {
            count: true,
            ..fixed_options()
        };
        let count_matcher = Matcher::new(&count_options).unwrap();
        let result = scan(
            Cursor::new(b"miss\n"),
            b"stdin",
            false,
            &count_options,
            &count_matcher,
            &mut BrokenOutput,
        );
        assert!(matches!(result, Err(ScanFailure::Output(error, false))
                         if error.kind() == io::ErrorKind::BrokenPipe));
    }
}
