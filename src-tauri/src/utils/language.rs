/*!
 * Language management module
 *
 * Provides global language setting management, supporting Chinese and English.
 * Uses thread-safe global state to manage current language settings.
 */

use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

/// Supported language types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Language {
    /// Simplified Chinese
    #[default]
    ZhCN,
    /// American English
    EnUS,
}

impl Language {
    /// Parse language type from string
    ///
    /// # Arguments
    /// * `s` - Language string, e.g., "zh-CN", "en-US"
    ///
    /// # Returns
    /// Corresponding language type, defaults to Chinese if unrecognized
    pub fn from_tag_lossy(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "zh-cn" | "zh" | "chinese" | "中文" => Language::ZhCN,
            "en-us" | "en" | "english" | "英文" => Language::EnUS,
            _ => Language::ZhCN, // Default to Chinese
        }
    }

    /// Get language identifier (BCP-47 tag)
    pub fn tag(&self) -> &'static str {
        match self {
            Language::ZhCN => "zh-CN",
            Language::EnUS => "en-US",
        }
    }

    /// Get localized display name of language
    pub fn display_name(&self) -> &'static str {
        match self {
            Language::ZhCN => "简体中文",
            Language::EnUS => "English",
        }
    }

    /// Get all supported languages
    pub fn all() -> Vec<Language> {
        vec![Language::ZhCN, Language::EnUS]
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.tag())
    }
}

/// Global language state
static CURRENT_LANGUAGE: LazyLock<std::sync::RwLock<Language>> =
    LazyLock::new(|| std::sync::RwLock::new(Language::ZhCN));

/// Language manager
///
/// Provides global language setting and retrieval functions, ensuring thread safety.
pub struct LanguageManager;

impl LanguageManager {
    /// Set current language
    ///
    /// # Arguments
    /// * `lang` - Language to set
    ///
    /// # Returns
    /// Returns true on success, false on failure
    pub fn set_language(lang: Language) -> bool {
        match CURRENT_LANGUAGE.write() {
            Ok(mut current_lang) => {
                *current_lang = lang;
                true
            }
            Err(_) => false,
        }
    }

    /// Get current language
    ///
    /// # Returns
    /// Currently set language, returns default language (Chinese) on failure
    pub fn get_language() -> Language {
        CURRENT_LANGUAGE
            .read()
            .map(|lang| *lang)
            .unwrap_or_default()
    }

    /// Set language from string
    ///
    /// # Arguments
    /// * `lang_str` - Language string
    ///
    /// # Returns
    /// Returns true on success, false on failure
    pub fn set_language_from_tag_lossy(lang_str: &str) -> bool {
        let lang = Language::from_tag_lossy(lang_str);
        Self::set_language(lang)
    }

    /// Get string representation of current language
    pub fn get_language_string() -> String {
        Self::get_language().tag().to_string()
    }

    /// Check if current language is Chinese
    pub fn is_chinese() -> bool {
        matches!(Self::get_language(), Language::ZhCN)
    }

    /// Check if current language is English
    pub fn is_english() -> bool {
        matches!(Self::get_language(), Language::EnUS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_from_str() {
        assert_eq!(Language::from_tag_lossy("zh-CN"), Language::ZhCN);
        assert_eq!(Language::from_tag_lossy("zh"), Language::ZhCN);
        assert_eq!(Language::from_tag_lossy("chinese"), Language::ZhCN);
        assert_eq!(Language::from_tag_lossy("中文"), Language::ZhCN);

        assert_eq!(Language::from_tag_lossy("en-US"), Language::EnUS);
        assert_eq!(Language::from_tag_lossy("en"), Language::EnUS);
        assert_eq!(Language::from_tag_lossy("english"), Language::EnUS);

        // Default case
        assert_eq!(Language::from_tag_lossy("unknown"), Language::ZhCN);
    }

    #[test]
    fn test_language_to_string() {
        assert_eq!(Language::ZhCN.tag(), "zh-CN");
        assert_eq!(Language::EnUS.tag(), "en-US");
    }

    #[test]
    fn test_language_manager() {
        // Test setting and getting
        assert!(LanguageManager::set_language(Language::EnUS));
        assert_eq!(LanguageManager::get_language(), Language::EnUS);
        assert!(LanguageManager::is_english());
        assert!(!LanguageManager::is_chinese());

        // Restore default
        assert!(LanguageManager::set_language(Language::ZhCN));
        assert_eq!(LanguageManager::get_language(), Language::ZhCN);
        assert!(LanguageManager::is_chinese());
        assert!(!LanguageManager::is_english());
    }

    #[test]
    fn test_language_manager_from_str() {
        assert!(LanguageManager::set_language_from_tag_lossy("en-US"));
        assert_eq!(LanguageManager::get_language(), Language::EnUS);

        assert!(LanguageManager::set_language_from_tag_lossy("zh-CN"));
        assert_eq!(LanguageManager::get_language(), Language::ZhCN);
    }

    #[test]
    fn test_language_all() {
        let langs = Language::all();
        assert_eq!(langs.len(), 2);
        assert!(langs.contains(&Language::ZhCN));
        assert!(langs.contains(&Language::EnUS));
    }
}
