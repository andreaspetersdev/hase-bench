use std::ffi::{OsStr, OsString};
use std::fmt;
use std::path::PathBuf;

use crate::config::{FilterDirective, FilterFile, FilterFileKind, FilterRule};

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
    pub delete_excluded: bool,
    pub checksum: bool,
    pub whole_file: bool,
    pub verbose: u8,
    pub preserve_times: bool,
    pub preserve_permissions: bool,
    pub filters: Vec<FilterDirective>,
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
    InvalidFilter(String),
    NonUtf8OptionValue(&'static str),
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
            Self::InvalidFilter(rule) => write!(formatter, "invalid filter rule: {rule}"),
            Self::NonUtf8OptionValue(option) => write!(formatter, "{option} requires UTF-8 text"),
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
            options.preserve_times = true;
            options.preserve_permissions = true;
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
        } else if options_enabled && argument == OsStr::new("--delete-excluded") {
            options.delete = true;
            options.delete_excluded = true;
        } else if options_enabled
            && (argument == OsStr::new("-c") || argument == OsStr::new("--checksum"))
        {
            options.checksum = true;
        } else if options_enabled
            && (argument == OsStr::new("-W") || argument == OsStr::new("--whole-file"))
        {
            options.whole_file = true;
        } else if options_enabled && argument == OsStr::new("-v") {
            options.verbose = options.verbose.saturating_add(1);
        } else if options_enabled
            && (argument == OsStr::new("-t") || argument == OsStr::new("--times"))
        {
            options.preserve_times = true;
        } else if options_enabled
            && (argument == OsStr::new("-p") || argument == OsStr::new("--perms"))
        {
            options.preserve_permissions = true;
        } else if options_enabled && argument == OsStr::new("--include") {
            options.filters.push(FilterDirective::Rule(FilterRule {
                include: true,
                pattern: next_utf8(&mut arguments, "--include")?,
            }));
        } else if options_enabled && argument == OsStr::new("--exclude") {
            options.filters.push(FilterDirective::Rule(FilterRule {
                include: false,
                pattern: next_utf8(&mut arguments, "--exclude")?,
            }));
        } else if options_enabled && argument == OsStr::new("--filter") {
            let rule = next_utf8(&mut arguments, "--filter")?;
            options.filters.push(parse_filter_directive(rule)?);
        } else if options_enabled && argument == OsStr::new("--include-from") {
            options.filters.push(FilterDirective::File(FilterFile {
                kind: FilterFileKind::Include,
                path: PathBuf::from(next_value(&mut arguments, "--include-from")?),
            }));
        } else if options_enabled && argument == OsStr::new("--exclude-from") {
            options.filters.push(FilterDirective::File(FilterFile {
                kind: FilterFileKind::Exclude,
                path: PathBuf::from(next_value(&mut arguments, "--exclude-from")?),
            }));
        } else if options_enabled && long_value(&argument, "--include=").is_some() {
            options.filters.push(FilterDirective::Rule(FilterRule {
                include: true,
                pattern: long_value(&argument, "--include=")
                    .expect("checked")
                    .to_owned(),
            }));
        } else if options_enabled && long_value(&argument, "--exclude=").is_some() {
            options.filters.push(FilterDirective::Rule(FilterRule {
                include: false,
                pattern: long_value(&argument, "--exclude=")
                    .expect("checked")
                    .to_owned(),
            }));
        } else if options_enabled && long_value(&argument, "--filter=").is_some() {
            options.filters.push(parse_filter_directive(
                long_value(&argument, "--filter=")
                    .expect("checked")
                    .to_owned(),
            )?);
        } else if options_enabled
            && (argument == OsStr::new("-e") || argument == OsStr::new("--rsh"))
        {
            options.remote_shell = Some(
                arguments
                    .next()
                    .ok_or(ParseError::MissingOptionValue("--rsh"))?,
            );
        } else if options_enabled && argument.to_string_lossy().starts_with('-') {
            parse_short_cluster(&argument, &mut options)?;
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

fn parse_short_cluster(argument: &OsStr, options: &mut Options) -> Result<(), ParseError> {
    let Some(cluster) = argument.to_str().and_then(|text| text.strip_prefix('-')) else {
        return Err(ParseError::UnknownOption(argument.to_os_string()));
    };
    if cluster.is_empty() || cluster.starts_with('-') {
        return Err(ParseError::UnknownOption(argument.to_os_string()));
    }
    for option in cluster.chars() {
        match option {
            'a' => {
                options.archive = true;
                options.recursive = true;
                options.preserve_times = true;
                options.preserve_permissions = true;
            }
            'r' => options.recursive = true,
            'n' => options.dry_run = true,
            'c' => options.checksum = true,
            'W' => options.whole_file = true,
            'v' => options.verbose = options.verbose.saturating_add(1),
            't' => options.preserve_times = true,
            'p' => options.preserve_permissions = true,
            _ => return Err(ParseError::UnknownOption(argument.to_os_string())),
        }
    }
    Ok(())
}

fn next_value<I>(arguments: &mut I, option: &'static str) -> Result<OsString, ParseError>
where
    I: Iterator<Item = OsString>,
{
    arguments
        .next()
        .ok_or(ParseError::MissingOptionValue(option))
}

fn next_utf8<I>(arguments: &mut I, option: &'static str) -> Result<String, ParseError>
where
    I: Iterator<Item = OsString>,
{
    next_value(arguments, option)?
        .into_string()
        .map_err(|_| ParseError::NonUtf8OptionValue(option))
}

fn long_value<'a>(argument: &'a OsStr, prefix: &str) -> Option<&'a str> {
    argument.to_str()?.strip_prefix(prefix)
}

fn parse_filter_directive(rule: String) -> Result<FilterDirective, ParseError> {
    let trimmed = rule.trim_start();
    let (include, pattern) = if let Some(pattern) = trimmed.strip_prefix("+ ") {
        (true, pattern)
    } else if let Some(pattern) = trimmed.strip_prefix("- ") {
        (false, pattern)
    } else if let Some(path) = trimmed
        .strip_prefix(". ")
        .or_else(|| trimmed.strip_prefix("merge "))
    {
        if path.is_empty() {
            return Err(ParseError::InvalidFilter(rule));
        }
        return Ok(FilterDirective::File(FilterFile {
            kind: FilterFileKind::Merge,
            path: PathBuf::from(path),
        }));
    } else {
        return Err(ParseError::InvalidFilter(rule));
    };
    if pattern.is_empty() {
        return Err(ParseError::InvalidFilter(rule));
    }
    Ok(FilterDirective::Rule(FilterRule {
        include,
        pattern: pattern.to_owned(),
    }))
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
