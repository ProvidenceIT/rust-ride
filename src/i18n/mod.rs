//! Internationalization module for multi-language support.
//!
//! Provides translation services using a simple key-value approach for runtime language switching.

pub mod loader;

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

// Re-export types
pub use loader::TranslationLoader;

/// Supported languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Language {
    #[default]
    English,
    Spanish,
    French,
    German,
    Italian,
}

impl Language {
    /// Get the language identifier string.
    pub fn id(&self) -> &'static str {
        match self {
            Language::English => "en-US",
            Language::Spanish => "es",
            Language::French => "fr",
            Language::German => "de",
            Language::Italian => "it",
        }
    }

    /// Get the display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Language::English => "English",
            Language::Spanish => "Español",
            Language::French => "Français",
            Language::German => "Deutsch",
            Language::Italian => "Italiano",
        }
    }

    /// Parse from a language identifier.
    pub fn from_id(id: &str) -> Option<Self> {
        let id = id.to_lowercase();
        if id.starts_with("en") {
            Some(Language::English)
        } else if id.starts_with("es") {
            Some(Language::Spanish)
        } else if id.starts_with("fr") {
            Some(Language::French)
        } else if id.starts_with("de") {
            Some(Language::German)
        } else if id.starts_with("it") {
            Some(Language::Italian)
        } else {
            None
        }
    }

    /// Get all supported languages.
    pub fn all() -> &'static [Language] {
        &[
            Language::English,
            Language::Spanish,
            Language::French,
            Language::German,
            Language::Italian,
        ]
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Simple translation store using key-value pairs.
struct TranslationStore {
    /// Current language
    current_language: Language,
    /// Translations per language (language -> key -> value)
    translations: HashMap<Language, HashMap<String, String>>,
}

impl TranslationStore {
    fn new() -> Self {
        let mut store = Self {
            current_language: Language::English,
            translations: HashMap::new(),
        };
        store.load_all_translations();
        store
    }

    fn load_all_translations(&mut self) {
        for lang in Language::all() {
            let translations = Self::parse_ftl(Self::get_ftl_content(*lang));
            self.translations.insert(*lang, translations);
        }
    }

    fn get_ftl_content(lang: Language) -> &'static str {
        match lang {
            Language::English => include_str!("locales/en-US/main.ftl"),
            Language::Spanish => include_str!("locales/es/main.ftl"),
            Language::French => include_str!("locales/fr/main.ftl"),
            Language::German => include_str!("locales/de/main.ftl"),
            Language::Italian => include_str!("locales/it/main.ftl"),
        }
    }

    fn parse_ftl(content: &str) -> HashMap<String, String> {
        let mut map = HashMap::new();
        for line in content.lines() {
            let line = line.trim();
            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            // Parse key = value
            if let Some((key, value)) = line.split_once('=') {
                map.insert(key.trim().to_string(), value.trim().to_string());
            }
        }
        map
    }

    fn translate(&self, key: &str) -> String {
        // Try current language
        if let Some(translations) = self.translations.get(&self.current_language) {
            if let Some(value) = translations.get(key) {
                return value.clone();
            }
        }

        // Fall back to English
        if self.current_language != Language::English {
            if let Some(translations) = self.translations.get(&Language::English) {
                if let Some(value) = translations.get(key) {
                    return value.clone();
                }
            }
        }

        // Return key as fallback
        key.to_string()
    }
}

/// Global translation store.
static TRANSLATION_STORE: OnceLock<Mutex<TranslationStore>> = OnceLock::new();

fn store() -> &'static Mutex<TranslationStore> {
    TRANSLATION_STORE.get_or_init(|| Mutex::new(TranslationStore::new()))
}

/// Initialize the translation system.
pub fn init() {
    let _ = store();
}

/// Translate a message by key.
pub fn t(key: &str) -> String {
    store().lock().unwrap().translate(key)
}

/// Translate a message with argument substitution.
/// Arguments are provided as key-value pairs and substituted for `{ $key }` patterns.
pub fn t_args(key: &str, args: &[(&str, &str)]) -> String {
    let mut result = t(key);
    for (arg_key, arg_value) in args {
        let pattern = format!("{{ ${} }}", arg_key);
        result = result.replace(&pattern, arg_value);
        // Also try without spaces
        let pattern_no_space = format!("{{${}}}", arg_key);
        result = result.replace(&pattern_no_space, arg_value);
    }
    result
}

/// Get the current language.
pub fn current_language() -> Language {
    store().lock().unwrap().current_language
}

/// Set the current language.
pub fn set_language(lang: Language) {
    store().lock().unwrap().current_language = lang;
}

/// Detect the system locale and return the best matching language.
pub fn detect_system_locale() -> Language {
    if let Some(locale) = sys_locale::get_locale() {
        if let Some(lang) = Language::from_id(&locale) {
            return lang;
        }
    }
    Language::English
}

/// Macro for convenient translation.
#[macro_export]
macro_rules! t {
    ($key:expr) => {
        $crate::i18n::t($key)
    };
    ($key:expr, $($arg_name:expr => $arg_value:expr),+ $(,)?) => {
        $crate::i18n::t_args($key, &[$(($arg_name, &$arg_value.to_string())),+])
    };
}

/// Translation service for the application (for non-global usage).
pub struct TranslationService {
    /// Current language
    current_language: Language,
    /// Translations per language
    translations: HashMap<Language, HashMap<String, String>>,
}

impl Default for TranslationService {
    fn default() -> Self {
        Self::new()
    }
}

impl TranslationService {
    /// Create a new translation service.
    pub fn new() -> Self {
        let mut service = Self {
            current_language: Language::English,
            translations: HashMap::new(),
        };
        service.initialize();
        service
    }

    /// Initialize with translations.
    pub fn initialize(&mut self) {
        for lang in Language::all() {
            let content = match lang {
                Language::English => include_str!("locales/en-US/main.ftl"),
                Language::Spanish => include_str!("locales/es/main.ftl"),
                Language::French => include_str!("locales/fr/main.ftl"),
                Language::German => include_str!("locales/de/main.ftl"),
                Language::Italian => include_str!("locales/it/main.ftl"),
            };
            let translations = TranslationStore::parse_ftl(content);
            self.translations.insert(*lang, translations);
        }
    }

    /// Get the current language.
    pub fn language(&self) -> Language {
        self.current_language
    }

    /// Set the current language.
    pub fn set_language(&mut self, lang: Language) {
        self.current_language = lang;
    }

    /// Translate a message by key.
    pub fn translate(&self, key: &str) -> String {
        // Try current language
        if let Some(translations) = self.translations.get(&self.current_language) {
            if let Some(value) = translations.get(key) {
                return value.clone();
            }
        }

        // Fall back to English
        if self.current_language != Language::English {
            if let Some(translations) = self.translations.get(&Language::English) {
                if let Some(value) = translations.get(key) {
                    return value.clone();
                }
            }
        }

        // Return key as fallback
        key.to_string()
    }

    /// Translate a message with arguments.
    pub fn translate_with_args(&self, key: &str, args: &[(&str, &str)]) -> String {
        let mut result = self.translate(key);
        for (arg_key, arg_value) in args {
            let pattern = format!("{{ ${} }}", arg_key);
            result = result.replace(&pattern, arg_value);
        }
        result
    }
}
