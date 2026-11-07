use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Theme {
    Light,
    Dark,
    #[default]
    System,
}
impl Theme {
    pub fn is_dark(self, system_dark: bool) -> bool {
        match self {
            Self::Light => false,
            Self::Dark => true,
            Self::System => system_dark,
        }
    }
    pub fn index(self) -> usize {
        match self {
            Self::Light => 0,
            Self::Dark => 1,
            Self::System => 2,
        }
    }
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => Self::Light,
            1 => Self::Dark,
            _ => Self::System,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Language {
    #[default]
    En,
    Ru,
}
impl Language {
    pub fn code(self) -> &'static str {
        self.choose("en", "ru")
    }
    pub fn choose<'a>(self, en: &'a str, ru: &'a str) -> &'a str {
        match self {
            Self::En => en,
            Self::Ru => ru,
        }
    }
    pub fn index(self) -> usize {
        usize::from(self == Self::Ru)
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Settings {
    pub theme: Theme,
    pub language: Language,
}
impl Settings {
    pub fn parse(text: &str) -> Self {
        let mut settings = Self::default();
        for line in text.lines() {
            if let Some((key, value)) = line.trim().split_once('=') {
                match key.trim() {
                    "theme" => {
                        settings.theme = match value.trim() {
                            "light" => Theme::Light,
                            "dark" => Theme::Dark,
                            _ => Theme::System,
                        }
                    }
                    "language" => {
                        settings.language = match value.trim() {
                            "ru" => Language::Ru,
                            _ => Language::En,
                        }
                    }
                    _ => {}
                }
            }
        }
        settings
    }
    pub fn load(path: &Path) -> Self {
        fs::read_to_string(path)
            .ok()
            .filter(|s| s.len() <= 4096)
            .map(|s| Self::parse(&s))
            .unwrap_or_default()
    }
    pub fn save(self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let theme = match self.theme {
            Theme::Light => "light",
            Theme::Dark => "dark",
            Theme::System => "system",
        };
        let language = self.language.choose("en", "ru");
        let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
        fs::write(&temporary, format!("theme={theme}\nlanguage={language}\n"))?;
        // std::fs::rename atomically replaces an existing file on Windows.
        fs::rename(&temporary, path).inspect_err(|_| {
            let _ = fs::remove_file(&temporary);
        })
    }
}
pub fn path() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|p| PathBuf::from(p).join("Solcalc").join("settings.ini"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn valid_unknown_and_partial_settings() {
        assert_eq!(
            Settings::parse("theme=dark\nlanguage=ru\n"),
            Settings {
                theme: Theme::Dark,
                language: Language::Ru
            }
        );
        assert_eq!(
            Settings::parse("theme=bad\nlanguage=bad\nfuture=1\nbroken"),
            Settings::default()
        );
        assert_eq!(
            Settings::parse("language=ru"),
            Settings {
                language: Language::Ru,
                ..Settings::default()
            }
        );
    }
    #[test]
    fn explicit_and_system_themes() {
        assert!(!Theme::Light.is_dark(true));
        assert!(Theme::Dark.is_dark(false));
        assert!(!Theme::System.is_dark(false));
        assert!(Theme::System.is_dark(true));
        for t in [Theme::Light, Theme::Dark, Theme::System] {
            assert_eq!(Theme::from_index(t.index()), t);
        }
    }
    #[test]
    fn settings_survive_save_reload_and_replacement() {
        let folder = std::env::current_dir()
            .unwrap()
            .join("qa")
            .join(format!("settings-test-{}", std::process::id()));
        let path = folder.join("settings.ini");
        let first = Settings {
            theme: Theme::Dark,
            language: Language::Ru,
        };
        first.save(&path).unwrap();
        assert_eq!(Settings::load(&path), first);
        let second = Settings {
            theme: Theme::Light,
            language: Language::En,
        };
        second.save(&path).unwrap();
        assert_eq!(Settings::load(&path), second);
        fs::remove_file(&path).unwrap();
        fs::remove_dir(&folder).unwrap();
    }
}
