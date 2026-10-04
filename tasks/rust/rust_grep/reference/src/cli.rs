use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct Options {
    pub patterns: Vec<Vec<u8>>,
    pub paths: Vec<PathBuf>,
    pub fixed: bool,
    pub insensitive: bool,
    pub invert: bool,
    pub word: bool,
    pub whole: bool,
    pub number: bool,
    pub force_filename: Option<bool>,
    pub count: bool,
    pub files_with: bool,
    pub files_without: bool,
    pub quiet: bool,
    pub only_matching: bool,
    pub recursive: bool,
    pub follow_links: bool,
    pub pattern_source: bool,
    pub before: usize,
    pub after: usize,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mode {
    Quiet,
    FilesWith,
    FilesWithout,
    Count,
    OnlyMatching,
    Normal,
}

impl Options {
    pub fn mode(&self) -> Mode {
        if self.quiet {
            Mode::Quiet
        } else if self.files_with {
            Mode::FilesWith
        } else if self.files_without {
            Mode::FilesWithout
        } else if self.count {
            Mode::Count
        } else if self.only_matching {
            Mode::OnlyMatching
        } else {
            Mode::Normal
        }
    }
}

pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Options, String> {
    let mut result = Options::default();
    let mut positional = Vec::new();
    let mut end_options = false;
    let mut args = args.into_iter();
    while let Some(raw) = args.next() {
        if !end_options && raw == OsStr::new("--") {
            end_options = true;
            continue;
        }
        let spelling = raw.to_str();
        if !end_options && spelling.is_some_and(|s| s.starts_with('-') && s != "-") {
            let token = spelling.unwrap();
            if let Some(pattern) = token.strip_prefix("--regexp=") {
                result.pattern_source = true;
                result.patterns.push(pattern.as_bytes().to_vec());
                continue;
            }
            if let Some(file) = token.strip_prefix("--file=") {
                result.pattern_source = true;
                read_patterns(PathBuf::from(file), &mut result.patterns)?;
                continue;
            }
            if token == "--include" || token == "--exclude" {
                let pattern = args
                    .next()
                    .ok_or_else(|| format!("{token} requires an argument"))?;
                let pattern = pattern.into_string().map_err(|_| "non-UTF-8 path filter")?;
                if token == "--include" {
                    result.include.push(pattern);
                } else {
                    result.exclude.push(pattern);
                }
                continue;
            }
            if let Some(pattern) = token.strip_prefix("--include=") {
                result.include.push(pattern.to_string());
                continue;
            }
            if let Some(pattern) = token.strip_prefix("--exclude=") {
                result.exclude.push(pattern.to_string());
                continue;
            }
            if token.starts_with("--") {
                return Err(format!("unsupported option {token}"));
            }
            let mut chars = token[1..].chars();
            while let Some(flag) = chars.next() {
                match flag {
                    'e' | 'f' => {
                        let tail: String = chars.collect();
                        let value = if tail.is_empty() {
                            args.next()
                                .ok_or_else(|| format!("-{flag} requires an argument"))?
                        } else {
                            OsString::from(tail)
                        };
                        if flag == 'e' {
                            result.pattern_source = true;
                            result.patterns.push(
                                value
                                    .to_str()
                                    .ok_or("non-UTF-8 pattern argument")?
                                    .as_bytes()
                                    .to_vec(),
                            );
                        } else {
                            result.pattern_source = true;
                            read_patterns(PathBuf::from(value), &mut result.patterns)?;
                        }
                        break;
                    }
                    'F' => result.fixed = true,
                    'E' => result.fixed = false,
                    'i' => result.insensitive = true,
                    'v' => result.invert = true,
                    'w' => result.word = true,
                    'x' => result.whole = true,
                    'n' => result.number = true,
                    'H' => result.force_filename = Some(true),
                    'h' => result.force_filename = Some(false),
                    'c' => result.count = true,
                    'l' => result.files_with = true,
                    'L' => result.files_without = true,
                    'q' => result.quiet = true,
                    'o' => result.only_matching = true,
                    'r' => result.recursive = true,
                    'R' => {
                        result.recursive = true;
                        result.follow_links = true;
                    }
                    'A' | 'B' | 'C' => {
                        let tail: String = chars.collect();
                        let value = if tail.is_empty() {
                            args.next()
                                .ok_or_else(|| format!("-{flag} requires an argument"))?
                        } else {
                            OsString::from(tail)
                        };
                        let raw = value.to_str().ok_or("non-UTF-8 context count")?;
                        if raw.is_empty() || !raw.bytes().all(|byte| byte.is_ascii_digit()) {
                            return Err(format!("invalid -{flag} count"));
                        }
                        let count = raw
                            .parse::<usize>()
                            .map_err(|_| format!("invalid -{flag} count"))?;
                        if count > 1000 {
                            return Err("context count exceeds 1000".to_string());
                        }
                        if flag != 'A' {
                            result.before = count;
                        }
                        if flag != 'B' {
                            result.after = count;
                        }
                        break;
                    }
                    _ => return Err(format!("unsupported option -{flag}")),
                }
            }
        } else {
            positional.push(raw);
        }
    }
    if !result.pattern_source {
        let first = positional.first().ok_or("missing pattern")?;
        result.patterns.push(
            first
                .to_str()
                .ok_or("non-UTF-8 pattern argument")?
                .as_bytes()
                .to_vec(),
        );
        positional.remove(0);
    }
    result.paths = positional.into_iter().map(PathBuf::from).collect();
    Ok(result)
}

fn read_patterns(path: PathBuf, patterns: &mut Vec<Vec<u8>>) -> Result<(), String> {
    let bytes = std::fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    if !bytes.is_empty() {
        let body = if bytes.ends_with(b"\n") {
            &bytes[..bytes.len() - 1]
        } else {
            &bytes[..]
        };
        for line in body.split(|byte| *byte == b'\n') {
            patterns.push(line.to_vec());
        }
    }
    Ok(())
}
