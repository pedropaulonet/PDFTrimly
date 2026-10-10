use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(usize)]
pub enum Language {
    #[default]
    English,
    Portuguese,
    French,
    Spanish,
    Italian,
}

const SOURCES: [&str; 5] = [
    include_str!("../i18n/en.toml"),
    include_str!("../i18n/pt-BR.toml"),
    include_str!("../i18n/fr.toml"),
    include_str!("../i18n/es.toml"),
    include_str!("../i18n/it.toml"),
];
type Catalog = BTreeMap<String, String>;
static CATALOGS: OnceLock<[Catalog; 5]> = OnceLock::new();

fn catalogs() -> &'static [Catalog; 5] {
    CATALOGS.get_or_init(|| {
        SOURCES.map(|source| toml::from_str(source).expect("valid embedded catalog"))
    })
}

fn placeholders(text: &str) -> BTreeSet<&str> {
    text.split('{')
        .skip(1)
        .filter_map(|part| part.split_once('}').map(|(key, _)| key))
        .collect()
}

pub fn validate_catalogs() -> Result<(), String> {
    let catalogs = catalogs();
    let reference = &catalogs[0];
    for (index, catalog) in catalogs.iter().enumerate() {
        if catalog.keys().ne(reference.keys()) {
            return Err(format!("Catalog {index} has missing or extra keys"));
        }
        for (key, text) in catalog {
            if text.is_empty() || placeholders(text) != placeholders(&reference[key]) {
                return Err(format!(
                    "Invalid translation or placeholders: {index}/{key}"
                ));
            }
        }
    }
    Ok(())
}

impl Language {
    pub fn from_index(index: i32) -> Self {
        match index {
            1 => Self::Portuguese,
            2 => Self::French,
            3 => Self::Spanish,
            4 => Self::Italian,
            _ => Self::English,
        }
    }

    pub fn from_locale(locale: &str) -> Option<Self> {
        let base = locale
            .split(['_', '-', '.', '@'])
            .next()?
            .to_ascii_lowercase();
        match base.as_str() {
            "en" => Some(Self::English),
            "pt" => Some(Self::Portuguese),
            "fr" => Some(Self::French),
            "es" => Some(Self::Spanish),
            "it" => Some(Self::Italian),
            _ => None,
        }
    }

    pub fn detect() -> Self {
        if let Ok(locale) = std::env::var("PDFTRIMLY_LANG") {
            return Self::from_locale(&locale).unwrap_or_default();
        }
        sys_locale::get_locales()
            .find_map(|locale| Self::from_locale(&locale))
            .unwrap_or_default()
    }

    pub fn text(self, key: &str) -> &'static str {
        catalogs()[self as usize]
            .get(key)
            .or_else(|| catalogs()[0].get(key))
            .expect("known catalog key")
    }

    // Substitute only the original template. A filename containing {total}, for
    // example, must never be interpreted as another placeholder.
    pub fn format(self, key: &str, args: &[(&str, String)]) -> String {
        let mut remaining = self.text(key);
        let mut output = String::new();
        while let Some((prefix, tail)) = remaining.split_once('{') {
            output.push_str(prefix);
            let Some((name, rest)) = tail.split_once('}') else {
                output.push('{');
                output.push_str(tail);
                return output;
            };
            if let Some((_, value)) = args.iter().find(|(key, _)| *key == name) {
                output.push_str(value);
            } else {
                output.push('{');
                output.push_str(name);
                output.push('}');
            }
            remaining = rest;
        }
        output.push_str(remaining);
        output
    }

    pub fn file_count(self, count: usize) -> String {
        self.format(
            if count == 1 {
                "files_one"
            } else {
                "files_other"
            },
            &[("count", count.to_string())],
        )
    }

    pub fn decimal(self, value: f64, precision: usize) -> String {
        let text = format!("{value:.precision$}");
        if self == Self::English {
            text
        } else {
            text.replace('.', ",")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_catalogs_have_identical_keys_and_placeholders() {
        validate_catalogs().unwrap();
    }

    #[test]
    fn locales_are_normalized_and_unknown_locales_fall_back() {
        for locale in ["pt_BR.UTF-8", "pt-PT", "PT_br@euro"] {
            assert_eq!(Language::from_locale(locale), Some(Language::Portuguese));
        }
        assert_eq!(Language::from_locale("fr_CA.UTF-8"), Some(Language::French));
        assert_eq!(Language::from_locale("C"), None);
        assert_eq!(Language::from_locale("de-DE"), None);
        assert_eq!(Language::from_index(99), Language::English);
    }

    #[test]
    fn interpolation_preserves_filenames_and_pluralization() {
        let text = Language::English.format(
            "processing",
            &[
                ("name", "{total}-scan.pdf".into()),
                ("current", "1".into()),
                ("total", "2".into()),
            ],
        );
        assert_eq!(text, "Processing 1 of 2: {total}-scan.pdf");
        assert_eq!(Language::English.file_count(1), "1 PDF selected");
        assert_eq!(Language::English.file_count(2), "2 PDFs selected");
        assert_eq!(Language::Portuguese.decimal(1.25, 2), "1,25");
    }
}
