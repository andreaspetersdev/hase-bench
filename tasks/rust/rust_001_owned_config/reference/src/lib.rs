#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    MissingEquals { line: usize },
    EmptyKey { line: usize },
    DuplicateKey { line: usize, key: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigEntry {
    key: String,
    value: String,
}

impl ConfigEntry {
    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigSnapshot {
    entries: Vec<ConfigEntry>,
}

impl ConfigSnapshot {
    pub fn parse(input: &str) -> Result<Self, ConfigError> {
        let mut entries: Vec<ConfigEntry> = Vec::new();

        for (index, physical_line) in input.split_inclusive('\n').enumerate() {
            let line_number = index + 1;
            let line = if let Some(without_lf) = physical_line.strip_suffix('\n') {
                without_lf.strip_suffix('\r').unwrap_or(without_lf)
            } else {
                physical_line
            };
            if trim_horizontal(line).is_empty() {
                continue;
            }

            let (raw_key, raw_value) = line
                .split_once('=')
                .ok_or(ConfigError::MissingEquals { line: line_number })?;
            let key = trim_horizontal(raw_key);
            let value = trim_horizontal(raw_value);
            if key.is_empty() {
                return Err(ConfigError::EmptyKey { line: line_number });
            }
            if entries
                .iter()
                .any(|entry| entry.key.eq_ignore_ascii_case(key))
            {
                return Err(ConfigError::DuplicateKey {
                    line: line_number,
                    key: key.to_owned(),
                });
            }
            entries.push(ConfigEntry {
                key: key.to_owned(),
                value: value.to_owned(),
            });
        }

        Ok(Self { entries })
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|entry| entry.key.eq_ignore_ascii_case(key))
            .map(|entry| entry.value.as_str())
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) -> Option<String> {
        let key = key.into();
        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| entry.key.eq_ignore_ascii_case(&key))
        {
            return Some(std::mem::replace(&mut entry.value, value.into()));
        }
        self.entries.push(ConfigEntry {
            key,
            value: value.into(),
        });
        None
    }

    pub fn entries(&self) -> &[ConfigEntry] {
        &self.entries
    }
}

fn trim_horizontal(text: &str) -> &str {
    text.trim_matches(|character| matches!(character, ' ' | '\t'))
}
