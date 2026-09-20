use std::ffi::{OsStr, OsString};
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathSpec {
    pub path: PathBuf,
    pub copy_contents: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Endpoint {
    Local(PathSpec),
    RemoteShell {
        host: OsString,
        path: PathSpec,
    },
    DaemonUrl(String),
    DaemonShell {
        host: OsString,
        module_path: PathSpec,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    pub archive: bool,
    pub recursive: bool,
    pub dry_run: bool,
    pub delete: bool,
    pub checksum: bool,
    pub remote_shell: Option<OsString>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    pub options: Options,
    pub sources: Vec<Endpoint>,
    pub destination: Endpoint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    MissingOperands,
    MissingOptionValue(&'static str),
    UnknownOption(OsString),
    NonUtf8DaemonUrl,
}

impl ParseError {
    pub fn exit_code(&self) -> u8 {
        2
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingOperands => {
                formatter.write_str("expected at least one source and one destination")
            }
            Self::MissingOptionValue(option) => write!(formatter, "{option} requires a value"),
            Self::UnknownOption(option) => write!(
                formatter,
                "unsupported option: {}",
                option.to_string_lossy()
            ),
            Self::NonUtf8DaemonUrl => formatter.write_str("daemon URLs must be valid UTF-8"),
        }
    }
}

impl std::error::Error for ParseError {}

pub fn parse_invocation<I, S>(arguments: I) -> Result<Invocation, ParseError>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let mut options = Options::default();
    let mut operands = Vec::new();
    let mut arguments = arguments.into_iter().map(Into::into);
    let mut options_enabled = true;
    while let Some(argument) = arguments.next() {
        if options_enabled && argument == OsStr::new("--") {
            options_enabled = false;
        } else if options_enabled && argument == OsStr::new("-a") {
            options.archive = true;
            options.recursive = true;
        } else if options_enabled
            && (argument == OsStr::new("-r") || argument == OsStr::new("--recursive"))
        {
            options.recursive = true;
        } else if options_enabled
            && (argument == OsStr::new("-n") || argument == OsStr::new("--dry-run"))
        {
            options.dry_run = true;
        } else if options_enabled && argument == OsStr::new("--delete") {
            options.delete = true;
        } else if options_enabled
            && (argument == OsStr::new("-c") || argument == OsStr::new("--checksum"))
        {
            options.checksum = true;
        } else if options_enabled
            && (argument == OsStr::new("-e") || argument == OsStr::new("--rsh"))
        {
            options.remote_shell = Some(
                arguments
                    .next()
                    .ok_or(ParseError::MissingOptionValue("--rsh"))?,
            );
        } else if options_enabled && argument.to_string_lossy().starts_with('-') {
            return Err(ParseError::UnknownOption(argument));
        } else {
            operands.push(argument);
        }
    }
    if operands.len() < 2 {
        return Err(ParseError::MissingOperands);
    }
    let destination = parse_endpoint(operands.pop().expect("length checked"))?;
    let sources = operands
        .into_iter()
        .map(parse_endpoint)
        .collect::<Result<_, _>>()?;
    Ok(Invocation {
        options,
        sources,
        destination,
    })
}

fn parse_endpoint(value: OsString) -> Result<Endpoint, ParseError> {
    let lossy = value.to_string_lossy();
    if lossy.starts_with("rsync://") {
        return value
            .into_string()
            .map(Endpoint::DaemonUrl)
            .map_err(|_| ParseError::NonUtf8DaemonUrl);
    }
    if is_windows_local_path(&lossy) {
        return Ok(Endpoint::Local(path_spec(value)));
    }
    if let Some((host, path)) = split_once(&value, "::") {
        return Ok(Endpoint::DaemonShell {
            host,
            module_path: path_spec(path),
        });
    }
    if let Some((host, path)) = split_once(&value, ":") {
        return Ok(Endpoint::RemoteShell {
            host,
            path: path_spec(path),
        });
    }
    Ok(Endpoint::Local(path_spec(value)))
}

fn path_spec(value: OsString) -> PathSpec {
    let copy_contents = value
        .to_string_lossy()
        .as_bytes()
        .last()
        .is_some_and(|last| *last == b'/' || *last == b'\\');
    PathSpec {
        path: PathBuf::from(value),
        copy_contents,
    }
}

fn is_windows_local_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    (bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':')
        || value.starts_with("\\\\")
        || value.starts_with("//")
}

fn split_once(value: &OsStr, delimiter: &str) -> Option<(OsString, OsString)> {
    let text = value.to_str()?;
    let (left, right) = text.split_once(delimiter)?;
    if left.is_empty() {
        return None;
    }
    Some((left.into(), right.into()))
}
