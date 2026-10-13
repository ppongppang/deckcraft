//! Interface text only. English command ids, saved documents and automation stay unchanged.
//! Catalogs are three-column TSV files with the same escaping rules as WordCraft.

mod catalog;

use std::cell::Cell;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum Language {
    #[default]
    Auto,
    En,
    Ko,
}

impl Language {
    pub const ALL: [Self; 3] = [Self::Auto, Self::En, Self::Ko];

    pub const fn code(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::En => "en",
            Self::Ko => "ko",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Auto => t("Automatic"),
            Self::En => "English",
            Self::Ko => "한국어",
        }
    }

    pub fn from_tag(tag: &str) -> Self {
        match tag.trim().split(['-', '_', '.', '@']).next().unwrap_or("").to_ascii_lowercase().as_str() {
            "ko" => Self::Ko,
            _ => Self::En,
        }
    }

    pub fn resolved(self) -> Self {
        if self != Self::Auto {
            return self;
        }
        // Headless tests and scripted snapshots retain their English default.
        #[cfg(test)]
        {
            Self::En
        }
        #[cfg(not(test))]
        {
            #[cfg(not(target_arch = "wasm32"))]
            let system = sys_locale::get_locale();
            #[cfg(target_arch = "wasm32")]
            let system = None;
            std::env::var("DECKCRAFT_LANGUAGE")
                .ok()
                .or(system)
                .map(|tag| Self::from_tag(&tag))
                .unwrap_or(Self::En)
        }
    }
}

impl From<String> for Language {
    fn from(value: String) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "en" => Self::En,
            "ko" => Self::Ko,
            _ => Self::Auto,
        }
    }
}

impl From<Language> for String {
    fn from(value: Language) -> Self {
        value.code().to_string()
    }
}

thread_local! {
    static CURRENT: Cell<Language> = const { Cell::new(Language::En) };
}

pub fn set_current(language: Language) {
    CURRENT.set(language.resolved());
}

fn korean() -> &'static catalog::Catalog {
    static CATALOG: OnceLock<catalog::Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let (catalog, errors) = catalog::Catalog::parse(include_str!("ko.tsv"));
        for error in errors {
            log::warn!("Korean interface catalog: {error}");
        }
        catalog
    })
}

/// Borrowed source strings remain borrowed; catalog strings live for the process lifetime.
pub fn tr(language: Language, source: &str) -> &str {
    if language.resolved() == Language::Ko {
        korean().in_context("", source).unwrap_or(source)
    } else {
        source
    }
}

pub fn t(source: &str) -> &str {
    CURRENT.with(|language| tr(language.get(), source))
}

/// Context distinguishes chart "Line" from a shape's line without changing model values.
pub fn tc<'a>(context: &str, source: &'a str) -> &'a str {
    CURRENT.with(|language| {
        if language.get().resolved() == Language::Ko {
            korean().in_context(context, source).unwrap_or(source)
        } else {
            source
        }
    })
}

pub fn owned(source: String) -> String {
    t(&source).to_string()
}

/// Replace already-formatted values in a translated template. Values are appended directly:
/// braces in a person's name, file name or document text are never interpreted recursively.
pub fn fmt(source: &str, values: &[(&str, String)]) -> String {
    let template = t(source);
    let mut result = String::new();
    let mut chars = template.char_indices().peekable();
    while let Some((start, ch)) = chars.next() {
        if ch == '{' {
            if chars.peek().is_some_and(|(_, c)| *c == '{') {
                chars.next();
                result.push('{');
                continue;
            }
            let mut end = None;
            for (index, next) in chars.by_ref() {
                if next == '}' {
                    end = Some(index + next.len_utf8());
                    break;
                }
            }
            if let Some(end) = end
                && let Some(marker) = template.get(start..end)
            {
                if let Some((_, value)) = values.iter().find(|(name, _)| *name == marker) {
                    result.push_str(value);
                } else {
                    result.push_str(marker);
                }
            } else if let Some(rest) = template.get(start..) {
                result.push_str(rest);
            }
        } else if ch == '}' && chars.peek().is_some_and(|(_, c)| *c == '}') {
            chars.next();
            result.push('}');
        } else {
            result.push(ch);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn korean_locale_and_saved_preference() {
        for tag in ["ko", "ko-KR", "ko_KR.UTF-8", "KO_kr"] {
            assert_eq!(Language::from_tag(tag), Language::Ko);
        }
        assert_eq!(Language::from_tag("en-US"), Language::En);
        assert_eq!(serde_json::from_str::<Language>("\"ko\"").unwrap(), Language::Ko);
        assert_eq!(serde_json::from_str::<Language>("\"future-language\"").unwrap(), Language::Auto);
        assert_eq!(serde_json::to_string(&Language::Ko).unwrap(), "\"ko\"");
    }

    #[test]
    fn translated_labels_keep_english_fallback() {
        assert_eq!(tr(Language::Ko, "Slide Master"), "슬라이드 마스터");
        assert_eq!(tr(Language::Ko, "Animations"), "애니메이션");
        assert_eq!(tr(Language::Ko, "not a catalog key"), "not a catalog key");
        assert_eq!(tr(Language::En, "Slide Master"), "Slide Master");
        set_current(Language::Ko);
        assert_eq!(t("Line"), "선");
        assert_eq!(tc("chart", "Line"), "꺾은선형");
        set_current(Language::En);
        let (_, errors) = catalog::parse_entries(include_str!("ko.tsv"));
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn translated_templates_reorder_values_without_reinterpreting_data() {
        set_current(Language::Ko);
        assert_eq!(fmt("Slide {0} of {1}", &[("{0}", "2".into()), ("{1}", "10".into())]), "전체 10장 중 2번째 슬라이드");
        let name = "문서 {error}";
        assert_eq!(fmt("unknown {path}: {error}", &[("{path}", name.into()), ("{error}", "실패".into())]), "unknown 문서 {error}: 실패");
        set_current(Language::En);
        assert_eq!(fmt("{{{name}}}", &[("{name}", "가".into())]), "{가}");
    }
}
