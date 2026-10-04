use crate::cli::Options;
use regex::bytes::{Regex, RegexBuilder};

pub struct Matcher {
    patterns: Vec<Pattern>,
    whole: bool,
    word: bool,
}

enum Pattern {
    Empty,
    Fixed { needle: Vec<u8>, insensitive: bool },
    Regex(Regex),
}

impl Matcher {
    pub fn new(options: &Options) -> Result<Self, String> {
        let mut patterns = Vec::new();
        for raw in &options.patterns {
            if raw.is_empty() {
                patterns.push(Pattern::Empty);
                continue;
            }
            if options.fixed {
                patterns.push(Pattern::Fixed {
                    needle: raw.clone(),
                    insensitive: options.insensitive,
                });
            } else {
                let source = std::str::from_utf8(raw).map_err(|_| "regex pattern is not UTF-8")?;
                validate_ere(source)?;
                let expression = if options.whole {
                    format!("^(?P<selected>{source})$")
                } else if options.word {
                    format!("(?:^|[^A-Za-z0-9_])(?P<selected>{source})(?:$|[^A-Za-z0-9_])")
                } else {
                    format!("(?P<selected>{source})")
                };
                let compiled = RegexBuilder::new(&expression)
                    .case_insensitive(options.insensitive)
                    .unicode(false)
                    .build()
                    .map_err(|error| format!("invalid regex: {error}"))?;
                patterns.push(Pattern::Regex(compiled));
            }
        }
        Ok(Self {
            patterns,
            whole: options.whole,
            word: options.word,
        })
    }

    pub fn matches(&self, line: &[u8]) -> bool {
        self.patterns.iter().any(|pattern| match pattern {
            Pattern::Empty => true,
            Pattern::Fixed {
                needle,
                insensitive,
            } => line
                .windows(needle.len())
                .enumerate()
                .any(|(start, window)| {
                    let same = if *insensitive {
                        window.eq_ignore_ascii_case(needle)
                    } else {
                        window == needle
                    };
                    same && self.accept_span(line, start, start + needle.len())
                }),
            Pattern::Regex(regex) => regex.is_match(line),
        })
    }

    pub fn spans(&self, line: &[u8]) -> Vec<(usize, usize)> {
        let mut spans = Vec::new();
        let mut cursor = 0;
        while cursor <= line.len() {
            let next = self
                .patterns
                .iter()
                .filter_map(|pattern| match pattern {
                    Pattern::Empty => None,
                    Pattern::Fixed {
                        needle,
                        insensitive,
                    } => {
                        if needle.len() > line.len().saturating_sub(cursor) {
                            return None;
                        }
                        line[cursor..].windows(needle.len()).enumerate().find_map(
                            |(offset, window)| {
                                let start = cursor + offset;
                                let same = if *insensitive {
                                    window.eq_ignore_ascii_case(needle)
                                } else {
                                    window == needle
                                };
                                (same && self.accept_span(line, start, start + needle.len()))
                                    .then_some((start, start + needle.len()))
                            },
                        )
                    }
                    Pattern::Regex(regex) => regex
                        .captures_at(line, cursor)
                        .and_then(|captures| captures.name("selected"))
                        .map(|selected| (selected.start(), selected.end())),
                })
                .min_by_key(|(start, _)| *start);
            let Some((start, end)) = next else { break };
            if end > start {
                spans.push((start, end));
                cursor = end;
            } else {
                cursor = start + 1;
            }
        }
        spans
    }

    fn accept_span(&self, line: &[u8], start: usize, end: usize) -> bool {
        if self.whole && (start != 0 || end != line.len()) {
            return false;
        }
        if self.word
            && (start > 0 && word_byte(line[start - 1]) || end < line.len() && word_byte(line[end]))
        {
            return false;
        }
        true
    }
}

fn validate_ere(source: &str) -> Result<(), String> {
    let bytes = source.as_bytes();
    let mut index = 0;
    let mut in_class = false;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => {
                index += 1;
                if index == bytes.len() || !b".\\^$*+?()[]{}|-".contains(&bytes[index]) {
                    return Err("unsupported regex escape".to_string());
                }
            }
            b'(' if !in_class && bytes.get(index + 1) == Some(&b'?') => {
                return Err("unsupported regex extension".to_string());
            }
            b'[' if in_class && bytes.get(index + 1) == Some(&b':') => {
                return Err("locale character classes are unsupported".to_string());
            }
            b'[' => in_class = true,
            b']' => in_class = false,
            _ => {}
        }
        index += 1;
    }
    Ok(())
}

fn word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}
