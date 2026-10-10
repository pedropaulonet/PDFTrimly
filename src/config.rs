//! Versioned UI defaults. No selections or document results are persisted.
use crate::i18n::Language;
use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preferences {
    pub language: Option<Language>,
    pub theme: Theme,
    pub level: i32,
    pub recursive: bool,
    pub folder: Option<PathBuf>,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            language: None,
            theme: Theme::System,
            level: 1,
            recursive: false,
            folder: None,
        }
    }
}
impl Preferences {
    pub fn language(&self) -> Language {
        if std::env::var_os("PDFTRIMLY_LANG").is_some() {
            Language::detect()
        } else {
            self.language.unwrap_or_else(Language::detect)
        }
    }
    pub fn theme_index(&self) -> i32 {
        match self.theme {
            Theme::System => 0,
            Theme::Light => 1,
            Theme::Dark => 2,
        }
    }
    pub fn parse(source: &str) -> Result<Self, String> {
        let value: toml::Table = toml::from_str(source).map_err(|e| format!("{e}"))?;
        if value.keys().any(|key| {
            ![
                "version",
                "language",
                "theme",
                "level",
                "folder",
                "recursive",
            ]
            .contains(&key.as_str())
        }) {
            return Err("Unknown preference field".into());
        }
        if value
            .get("version")
            .is_some_and(|v| v.as_integer() != Some(1))
        {
            return Err("Unsupported preference version".into());
        }
        let string = |key: &str, fallback: &str| -> Result<String, String> {
            value
                .get(key)
                .map(|v| {
                    v.as_str()
                        .map(str::to_owned)
                        .ok_or_else(|| format!("Invalid {key}"))
                })
                .unwrap_or_else(|| Ok(fallback.into()))
        };
        let language = match string("language", "system")?.as_str() {
            "system" => None,
            "en" => Some(Language::English),
            "pt-BR" => Some(Language::Portuguese),
            "fr" => Some(Language::French),
            "es" => Some(Language::Spanish),
            "it" => Some(Language::Italian),
            _ => return Err("Invalid language".into()),
        };
        let theme = match string("theme", "system")?.as_str() {
            "system" => Theme::System,
            "light" => Theme::Light,
            "dark" => Theme::Dark,
            _ => return Err("Invalid theme".into()),
        };
        let level = match string("level", "normal")?.as_str() {
            "light" => 0,
            "normal" => 1,
            "ultra" => 2,
            _ => return Err("Invalid level".into()),
        };
        let folder = string("folder", "")?;
        let folder = if folder.is_empty() {
            None
        } else {
            let p = PathBuf::from(folder);
            if !p.is_absolute() {
                return Err("Folder must be absolute".into());
            }
            Some(p)
        };
        let recursive = value
            .get("recursive")
            .map(|v| v.as_bool().ok_or("Invalid recursive option"))
            .transpose()?
            .unwrap_or(false);
        Ok(Self {
            language,
            theme,
            level,
            recursive,
            folder,
        })
    }
    fn encode(&self) -> io::Result<String> {
        let language = self
            .language
            .map(|l| ["en", "pt-BR", "fr", "es", "it"][l as usize])
            .unwrap_or("system");
        let mut table = toml::Table::new();
        table.insert("recursive".into(), self.recursive.into());
        table.insert("version".into(), 1.into());
        table.insert("language".into(), language.into());
        table.insert(
            "theme".into(),
            ["system", "light", "dark"][self.theme_index() as usize].into(),
        );
        let level = ["light", "normal", "ultra"]
            .get(self.level as usize)
            .ok_or_else(|| io::Error::other("Invalid level"))?;
        table.insert("level".into(), (*level).into());
        let folder = self
            .folder
            .as_ref()
            .map(|p| {
                p.to_str()
                    .ok_or_else(|| io::Error::other("Folder is not UTF-8"))
            })
            .transpose()?
            .unwrap_or("");
        table.insert("folder".into(), folder.into());
        toml::to_string(&table).map_err(io::Error::other)
    }
}

pub fn config_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(not(target_os = "windows"))]
    let base = unix_config_base(
        std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
        std::env::var_os("HOME").map(PathBuf::from),
    );
    base.filter(|p| p.is_absolute())
        .map(|p| p.join("pdftrimly").join("preferences.toml"))
}
#[cfg(any(not(target_os = "windows"), test))]
fn unix_config_base(xdg: Option<PathBuf>, home: Option<PathBuf>) -> Option<PathBuf> {
    xdg.filter(|p| p.is_absolute())
        .or_else(|| home.filter(|p| p.is_absolute()).map(|p| p.join(".config")))
}
pub fn load(path: Option<&Path>) -> (Preferences, Option<&'static str>) {
    let Some(path) = path else {
        return (Preferences::default(), Some("config_unavailable"));
    };
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return (Preferences::default(), None),
        Err(_) => return (Preferences::default(), Some("config_load_failed")),
    };
    let mut source = String::new();
    if file.take(65537).read_to_string(&mut source).is_err() || source.len() > 65536 {
        return (Preferences::default(), Some("config_load_failed"));
    }
    match Preferences::parse(&source) {
        Ok(p) => (p, None),
        Err(_) => (Preferences::default(), Some("config_load_failed")),
    }
}
pub fn save(path: &Path, preferences: &Preferences) -> io::Result<()> {
    let encoded = preferences.encode()?;
    if preferences
        .folder
        .as_ref()
        .is_some_and(|p| !p.is_absolute() || !p.is_dir())
    {
        return Err(io::Error::other("Default folder is unavailable"));
    }
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("Missing config directory"))?;
    fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(encoded.as_bytes())?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|e| e.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip_all_languages_themes_and_levels() {
        let dir = tempfile::tempdir().unwrap();
        for language in [
            None,
            Some(Language::English),
            Some(Language::Portuguese),
            Some(Language::French),
            Some(Language::Spanish),
            Some(Language::Italian),
        ] {
            for theme in [Theme::System, Theme::Light, Theme::Dark] {
                for level in 0..3 {
                    let p = Preferences {
                        language,
                        theme: theme.clone(),
                        level,
                        recursive: true,
                        folder: Some(dir.path().into()),
                    };
                    assert_eq!(Preferences::parse(&p.encode().unwrap()).unwrap(), p);
                }
            }
        }
    }
    #[test]
    fn missing_invalid_large_and_future_config_recover_without_overwriting() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("preferences.toml");
        assert_eq!(load(Some(&path)), (Preferences::default(), None));
        for text in [
            "broken[",
            "version = 2",
            "theme = 'purple'",
            "level = 3",
            "folder = 'relative'",
            "unknown = true",
            &" ".repeat(65537),
        ] {
            fs::write(&path, text).unwrap();
            assert!(load(Some(&path)).1.is_some());
            assert_eq!(fs::read_to_string(&path).unwrap(), text);
        }
    }
    #[test]
    fn save_is_replaced_and_failed_save_preserves_previous_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/preferences.toml");
        save(&path, &Preferences::default()).unwrap();
        let p = Preferences {
            level: 2,
            theme: Theme::Dark,
            ..Default::default()
        };
        save(&path, &p).unwrap();
        assert_eq!(load(Some(&path)).0, p);
        let before = fs::read(&path).unwrap();
        let invalid = Preferences {
            folder: Some(dir.path().join("missing")),
            ..p
        };
        assert!(save(&path, &invalid).is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
    }
    #[test]
    fn config_base_ignores_relative_xdg_and_requires_absolute_home() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        let xdg = dir.path().join("xdg");
        assert_eq!(
            unix_config_base(Some("relative".into()), Some(home.clone())),
            Some(home.join(".config"))
        );
        assert_eq!(unix_config_base(Some(xdg.clone()), None), Some(xdg));
        assert_eq!(unix_config_base(None, Some("relative".into())), None);
    }
    #[test]
    fn failed_atomic_replacement_removes_temporary_and_preserves_target() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("preferences.toml");
        fs::create_dir(&path).unwrap();
        fs::write(path.join("marker"), b"keep existing target").unwrap();
        assert!(save(&path, &Preferences::default()).is_err());
        assert_eq!(
            fs::read(path.join("marker")).unwrap(),
            b"keep existing target"
        );
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
