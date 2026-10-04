use crate::cli::Options;
use regex::Regex;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub enum Input {
    File(PathBuf),
    Error(String),
}

struct Glob {
    expression: Regex,
    basename_only: bool,
}

impl Glob {
    fn new(pattern: &str) -> Result<Self, String> {
        let pattern = pattern.replace('\\', "/");
        let mut expression = String::from("^");
        let mut characters = pattern.chars().peekable();
        while let Some(character) = characters.next() {
            match character {
                '*' if characters.peek() == Some(&'*') => {
                    characters.next();
                    expression.push_str(".*");
                }
                '*' => expression.push_str("[^/]*"),
                '?' => expression.push_str("[^/]"),
                other => expression.push_str(&regex::escape(&other.to_string())),
            }
        }
        expression.push('$');
        let expression = Regex::new(&expression).map_err(|error| error.to_string())?;
        Ok(Self {
            expression,
            basename_only: !pattern.contains('/'),
        })
    }

    fn matches(&self, relative: &Path) -> bool {
        let name = if self.basename_only {
            relative.file_name().and_then(|name| name.to_str())
        } else {
            relative.to_str()
        };
        name.is_some_and(|name| self.expression.is_match(&name.replace('\\', "/")))
    }
}

pub fn collect(options: &Options) -> Result<Vec<Input>, String> {
    if options.paths.is_empty() {
        return Ok(vec![Input::File(PathBuf::from("-"))]);
    }
    let includes = options
        .include
        .iter()
        .map(|pattern| Glob::new(pattern))
        .collect::<Result<Vec<_>, _>>()?;
    let excludes = options
        .exclude
        .iter()
        .map(|pattern| Glob::new(pattern))
        .collect::<Result<Vec<_>, _>>()?;
    let mut results = Vec::new();
    let mut visited = HashSet::new();
    for path in &options.paths {
        if path == Path::new("-") {
            results.push(Input::File(path.clone()));
            continue;
        }
        let metadata = if options.follow_links {
            path.metadata()
        } else {
            path.symlink_metadata()
        };
        match metadata {
            Ok(metadata) if metadata.is_dir() => visit(
                path,
                path,
                options,
                &includes,
                &excludes,
                &mut visited,
                &mut results,
            ),
            Ok(metadata) if metadata.is_file() => {
                let relative = Path::new(path.file_name().unwrap_or_default());
                if selected(relative, &includes, &excludes) {
                    results.push(Input::File(path.clone()));
                }
            }
            Ok(_) => results.push(Input::Error(format!(
                "{}: unsupported file type",
                path.display()
            ))),
            Err(error) => results.push(Input::Error(format!("{}: {error}", path.display()))),
        }
    }
    Ok(results)
}

fn visit(
    root: &Path,
    directory: &Path,
    options: &Options,
    includes: &[Glob],
    excludes: &[Glob],
    visited: &mut HashSet<String>,
    results: &mut Vec<Input>,
) {
    if !options.recursive {
        results.push(Input::Error(format!(
            "{}: is a directory",
            directory.display()
        )));
        return;
    }
    let relative = directory.strip_prefix(root).unwrap_or(directory);
    if !relative.as_os_str().is_empty() && excludes.iter().any(|filter| filter.matches(relative)) {
        return;
    }
    if options.follow_links {
        match directory_identity(directory) {
            Ok(identity) => {
                if !visited.insert(identity) {
                    return;
                }
            }
            Err(error) => {
                results.push(Input::Error(format!("{}: {error}", directory.display())));
                return;
            }
        }
    }
    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) => {
            results.push(Input::Error(format!("{}: {error}", directory.display())));
            return;
        }
    };
    let mut children = Vec::new();
    for entry in entries {
        match entry {
            Ok(entry) => children.push(entry.path()),
            Err(error) => results.push(Input::Error(format!("{}: {error}", directory.display()))),
        }
    }
    children.sort();
    for child in children {
        let metadata = if options.follow_links {
            child.metadata()
        } else {
            child.symlink_metadata()
        };
        match metadata {
            Ok(metadata) if metadata.is_dir() => {
                visit(root, &child, options, includes, excludes, visited, results);
            }
            Ok(metadata) if metadata.is_file() => {
                let relative = child.strip_prefix(root).unwrap_or(&child);
                if selected(relative, includes, excludes) {
                    results.push(Input::File(child));
                }
            }
            Ok(metadata) if metadata.file_type().is_symlink() && !options.follow_links => {}
            Ok(_) => results.push(Input::Error(format!(
                "{}: unsupported file type",
                child.display()
            ))),
            Err(error) => results.push(Input::Error(format!("{}: {error}", child.display()))),
        }
    }
}

#[cfg(unix)]
fn directory_identity(path: &Path) -> std::io::Result<String> {
    use std::os::unix::fs::MetadataExt;
    let metadata = path.metadata()?;
    Ok(format!("unix:{}:{}", metadata.dev(), metadata.ino()))
}

#[cfg(windows)]
fn directory_identity(path: &Path) -> std::io::Result<String> {
    Ok(format!("path:{}", path.canonicalize()?.display()))
}

fn selected(relative: &Path, includes: &[Glob], excludes: &[Glob]) -> bool {
    !excludes.iter().any(|filter| filter.matches(relative))
        && (includes.is_empty() || includes.iter().any(|filter| filter.matches(relative)))
}
