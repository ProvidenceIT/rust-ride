//! Translation file loader for runtime translation loading.

use std::collections::HashMap;
use std::path::Path;

/// Translation loader for loading .ftl files.
pub struct TranslationLoader {
    /// Loaded translation strings
    translations: HashMap<String, String>,
}

impl TranslationLoader {
    /// Create a new translation loader.
    pub fn new() -> Self {
        Self {
            translations: HashMap::new(),
        }
    }

    /// Load translations from a directory of .ftl files.
    pub fn load_from_dir(&mut self, dir: &Path) -> Result<(), TranslationLoadError> {
        if !dir.exists() || !dir.is_dir() {
            return Err(TranslationLoadError::DirectoryNotFound(
                dir.display().to_string(),
            ));
        }

        for entry in std::fs::read_dir(dir)
            .map_err(|e| TranslationLoadError::IoError(e.to_string()))?
        {
            let entry = entry.map_err(|e| TranslationLoadError::IoError(e.to_string()))?;
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "ftl") {
                self.load_file(&path)?;
            }
        }

        Ok(())
    }

    /// Load translations from a single .ftl file.
    pub fn load_file(&mut self, path: &Path) -> Result<(), TranslationLoadError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| TranslationLoadError::IoError(e.to_string()))?;

        self.parse_ftl(&content);
        Ok(())
    }

    /// Parse FTL content and extract key-value pairs.
    fn parse_ftl(&mut self, content: &str) {
        for line in content.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse key = value pairs
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim().to_string();
                let value = value.trim().to_string();
                self.translations.insert(key, value);
            }
        }
    }

    /// Get a translation by key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.translations.get(key).map(|s| s.as_str())
    }

    /// Get all loaded translations.
    pub fn translations(&self) -> &HashMap<String, String> {
        &self.translations
    }

    /// Get the number of loaded translations.
    pub fn count(&self) -> usize {
        self.translations.len()
    }
}

impl Default for TranslationLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur when loading translations.
#[derive(Debug, thiserror::Error)]
pub enum TranslationLoadError {
    #[error("Directory not found: {0}")]
    DirectoryNotFound(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Detect the best available locale from the system.
pub trait LocaleDetector {
    /// Get the system locale.
    fn system_locale(&self) -> Option<String>;

    /// Get the best matching supported locale.
    fn best_match(&self, supported: &[&str]) -> String;
}

/// Default locale detector using sys-locale.
#[derive(Default)]
pub struct SystemLocaleDetector;

impl LocaleDetector for SystemLocaleDetector {
    fn system_locale(&self) -> Option<String> {
        sys_locale::get_locale()
    }

    fn best_match(&self, supported: &[&str]) -> String {
        if let Some(locale) = self.system_locale() {
            // Try exact match first
            if supported.contains(&locale.as_str()) {
                return locale;
            }

            // Try language prefix match (e.g., "en" matches "en-US")
            let prefix = locale.split('-').next().unwrap_or(&locale);
            for supported_locale in supported {
                if supported_locale.starts_with(prefix) {
                    return (*supported_locale).to_string();
                }
            }
        }

        // Default to first supported locale (usually English)
        supported.first().unwrap_or(&"en-US").to_string()
    }
}
