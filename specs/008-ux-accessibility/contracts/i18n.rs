//! Internationalization Module Contract
//!
//! Public API for language/locale management and message translation.

use std::collections::HashMap;

// ============================================================================
// Translation Service
// ============================================================================

/// Core translation service using Fluent.
pub trait TranslationService {
    /// Get a translated message by key.
    fn get(&self, key: &str) -> String;

    /// Get a translated message with arguments.
    fn get_with_args(&self, key: &str, args: &HashMap<String, FluentValue>) -> String;

    /// Get a translated message with a single argument.
    fn get_with_arg(&self, key: &str, arg_name: &str, value: impl Into<FluentValue>) -> String;

    /// Check if a translation key exists.
    fn has_key(&self, key: &str) -> bool;

    /// Get the current locale.
    fn current_locale(&self) -> &str;

    /// Get all available locales.
    fn available_locales(&self) -> &[LocaleInfo];

    /// Set the active locale.
    fn set_locale(&mut self, locale: &str) -> Result<(), I18nError>;
}

/// Value types for Fluent message arguments.
#[derive(Clone, Debug)]
pub enum FluentValue {
    String(String),
    Number(f64),
}

impl From<&str> for FluentValue {
    fn from(s: &str) -> Self {
        FluentValue::String(s.to_string())
    }
}

impl From<String> for FluentValue {
    fn from(s: String) -> Self {
        FluentValue::String(s)
    }
}

impl From<i32> for FluentValue {
    fn from(n: i32) -> Self {
        FluentValue::Number(n as f64)
    }
}

impl From<f64> for FluentValue {
    fn from(n: f64) -> Self {
        FluentValue::Number(n)
    }
}

/// Information about an available locale.
#[derive(Clone, Debug)]
pub struct LocaleInfo {
    /// Locale code (e.g., "en-US", "es", "fr")
    pub code: String,

    /// Native name (e.g., "English", "Español", "Français")
    pub native_name: String,

    /// English name (e.g., "English", "Spanish", "French")
    pub english_name: String,

    /// Completion percentage (0-100)
    pub completion: u8,
}

// ============================================================================
// Locale Detection
// ============================================================================

/// System locale detection.
pub trait LocaleDetector {
    /// Detect the system locale.
    fn detect_system_locale(&self) -> Option<String>;

    /// Find the best matching supported locale for a requested locale.
    fn negotiate_locale(&self, requested: &str, available: &[String]) -> String;
}

// ============================================================================
// Translation Loader
// ============================================================================

/// Loads translation bundles from files or embedded resources.
pub trait TranslationLoader {
    /// Load translations for a locale.
    fn load(&self, locale: &str) -> Result<TranslationBundle, I18nError>;

    /// Get the path to translation files.
    fn translation_path(&self) -> &std::path::Path;
}

/// A bundle of translations for a single locale.
pub struct TranslationBundle {
    pub locale: String,
    pub messages: HashMap<String, String>,
}

// ============================================================================
// I18n Errors
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum I18nError {
    #[error("Locale not found: {0}")]
    LocaleNotFound(String),

    #[error("Failed to load translations: {0}")]
    LoadFailed(String),

    #[error("Invalid translation file: {0}")]
    ParseError(String),

    #[error("Missing required key: {0}")]
    MissingKey(String),
}

// ============================================================================
// Convenience Macros (for implementation)
// ============================================================================

/// Example usage in UI code:
/// ```rust
/// let label = t!("button.start_ride");
/// let msg = t!("ride.duration", "hours" => 1, "minutes" => 30);
/// ```
///
/// Macro implementation would be:
/// ```rust
/// macro_rules! t {
///     ($key:expr) => {
///         I18N.get($key)
///     };
///     ($key:expr, $($arg:expr => $val:expr),*) => {{
///         let mut args = std::collections::HashMap::new();
///         $(args.insert($arg.to_string(), FluentValue::from($val));)*
///         I18N.get_with_args($key, &args)
///     }};
/// }
/// ```

// ============================================================================
// Standard Translation Keys (for reference)
// ============================================================================

/// Common translation key namespaces:
/// - `app.*` - Application-level strings (name, version, about)
/// - `nav.*` - Navigation labels
/// - `button.*` - Button labels
/// - `ride.*` - Ride screen labels
/// - `workout.*` - Workout-related strings
/// - `settings.*` - Settings screen labels
/// - `accessibility.*` - Accessibility-specific strings
/// - `onboarding.*` - Onboarding wizard strings
/// - `error.*` - Error messages
/// - `glossary.*` - Cycling terminology definitions
