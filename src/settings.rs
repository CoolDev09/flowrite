use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub(super) struct Settings {
    pub version: u32,
    pub general: General,
    pub appearance: Appearance,
    pub editor: Editor,
    pub saving: Saving,
    pub citations: Citations,
    pub export: Export,
    pub proofing: Proofing,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            general: General::default(),
            appearance: Appearance::default(),
            editor: Editor::default(),
            saving: Saving::default(),
            citations: Citations::default(),
            export: Export::default(),
            proofing: Proofing::default(),
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub(super) struct General {
    pub restore_tabs: bool,
    pub folder: PathBuf,
    pub ask_export: bool,
    pub date_format: usize,
    pub author: String,
    pub instructor: String,
    pub course: String,
}
impl Default for General {
    fn default() -> Self {
        Self {
            restore_tabs: true,
            folder: std::env::var_os("USERPROFILE")
                .map(PathBuf::from)
                .unwrap_or_default()
                .join("Downloads"),
            ask_export: false,
            date_format: 0,
            author: String::new(),
            instructor: String::new(),
            course: String::new(),
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub(super) struct Appearance {
    pub dim_page: bool,
    pub accent: usize,
    pub scale: f32,
    pub list_view: bool,
    pub recent_limit: usize,
    pub thumbnails: bool,
    pub sidebar_width: f32,
    pub folders: bool,
    pub high_contrast: bool,
    pub hand_pointer: bool,
}
impl Default for Appearance {
    fn default() -> Self {
        Self {
            dim_page: false,
            accent: 0,
            scale: 1.0,
            list_view: false,
            recent_limit: 12,
            thumbnails: true,
            sidebar_width: 222.0,
            folders: true,
            high_contrast: false,
            hand_pointer: true,
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub(super) struct Editor {
    pub font: String,
    pub size: f32,
    pub spacing: f32,
    pub margin: f32,
    pub markdown: bool,
    pub slash: bool,
    pub handles: bool,
    pub words: bool,
    pub characters: bool,
    pub reading_time: bool,
    pub word_goal: usize,
}
impl Default for Editor {
    fn default() -> Self {
        Self {
            font: "Liberation Serif".into(),
            size: 12.0,
            spacing: 1.15,
            margin: 1.0,
            markdown: true,
            slash: true,
            handles: true,
            words: true,
            characters: true,
            reading_time: true,
            word_goal: 0,
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub(super) struct Saving {
    pub delay: f64,
    pub indicator: bool,
}
impl Default for Saving {
    fn default() -> Self {
        Self {
            delay: 1.5,
            indicator: true,
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub(super) struct Citations {
    pub heading: String,
    pub alphabetize: bool,
    pub hanging_inches: f32,
    pub running_header: bool,
}
impl Default for Citations {
    fn default() -> Self {
        Self {
            heading: String::new(),
            alphabetize: true,
            hanging_inches: 0.5,
            running_header: true,
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub(super) struct Export {
    pub toast: bool,
    pub toast_seconds: f64,
    pub auto_open: bool,
    pub page_colors: bool,
    pub include_images: bool,
    pub page_numbers: bool,
    pub a4: bool,
}
impl Default for Export {
    fn default() -> Self {
        Self {
            toast: true,
            toast_seconds: 4.0,
            auto_open: false,
            page_colors: false,
            include_images: true,
            page_numbers: true,
            a4: false,
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub(super) struct Proofing {
    pub spelling: bool,
    pub grammar: bool,
    pub highlight_only: bool,
    pub as_you_type: bool,
    pub debounce: f64,
    pub dotted: bool,
    pub opacity: f32,
    pub spelling_color: [u8; 3],
    pub grammar_color: [u8; 3],
    pub skip_caps: bool,
    pub skip_numbers: bool,
    pub categories: [bool; 6],
    pub ignored_words: Vec<String>,
    pub ignored_rules: Vec<String>,
    pub dictionary: Vec<String>,
}
impl Default for Proofing {
    fn default() -> Self {
        Self {
            spelling: true,
            grammar: true,
            highlight_only: false,
            as_you_type: true,
            debounce: 0.4,
            dotted: false,
            opacity: 0.9,
            spelling_color: [235, 107, 107],
            grammar_color: [113, 174, 232],
            skip_caps: true,
            skip_numbers: true,
            categories: [true, true, true, true, true, false],
            ignored_words: Vec::new(),
            ignored_rules: Vec::new(),
            dictionary: Vec::new(),
        }
    }
}

impl Settings {
    pub fn load(directory: &Path) -> (Self, Option<String>) {
        let path = directory.join("settings.json");
        if !path.exists() {
            return (Self::default(), None);
        }
        if std::fs::read(&path)
            .ok()
            .is_some_and(|bytes| newer_version(&bytes))
        {
            return (Self::default(), Some("Settings from a newer version were left untouched. Using defaults for this session.".into()));
        }
        let parsed = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Self>(&bytes).ok());
        match parsed {
            Some(settings) if settings.version<=1 => (settings,None),
            Some(_) => (Self::default(),Some("Settings from a newer version were left untouched. Using defaults for this session.".into())),
            None => {
                // Preserve corrupt input before the next auto-save can replace it.
                let mut corrupt=directory.join("settings.json.corrupt");if corrupt.exists(){corrupt=directory.join(format!("settings.json.{}.corrupt",super::workspace::timestamp()));}
                if let Err(error)=std::fs::rename(&path,&corrupt) {
                    return (Self::default(),Some(format!("Could not preserve unreadable settings: {error}. Settings have been left unchanged.")));
                }
                let backup=std::fs::read(directory.join("settings.json.bak")).ok().and_then(|b|serde_json::from_slice::<Self>(&b).ok()).filter(|s|s.version<=1);
                (backup.unwrap_or_default(),Some(format!("Recovered settings; the unreadable file is kept at {}.",corrupt.display())))
            }
        }
    }
    pub fn save(&self, directory: &Path) -> Result<(), String> {
        let path = directory.join("settings.json");
        if path.exists() {
            let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
            if newer_version(&bytes) {
                return Err("Newer settings have not been overwritten.".into());
            }
            let _: Self = serde_json::from_slice(&bytes).map_err(|_| {
                "Unreadable settings have not been overwritten. Restart to recover them.".to_owned()
            })?;
        }
        let content = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        super::workspace::write_atomic(&path, &content).map_err(|e| e.to_string())
    }
}

fn newer_version(bytes: &[u8]) -> bool {
    serde_json::from_slice::<serde_json::Value>(bytes)
        .ok()
        .and_then(|value| value.get("version").and_then(|v| v.as_u64()))
        .is_some_and(|version| version > 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_object_uses_defaults_and_round_trips() {
        let settings: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(settings, Settings::default());
        assert_eq!(
            serde_json::from_slice::<Settings>(&serde_json::to_vec(&settings).unwrap()).unwrap(),
            settings
        );
    }
    #[test]
    fn broken_settings_are_preserved() {
        let dir = std::env::temp_dir().join(format!(
            "flowrite-settings-{}",
            super::super::workspace::timestamp()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("settings.json"), b"broken").unwrap();
        let (settings, notice) = Settings::load(&dir);
        assert_eq!(settings, Settings::default());
        assert!(notice.is_some());
        assert_eq!(
            std::fs::read(dir.join("settings.json.corrupt")).unwrap(),
            b"broken"
        );
    }

    #[test]
    fn settings_backup_survives_recovery_and_next_save() {
        let dir = std::env::temp_dir().join(format!(
            "flowrite-settings-backup-{}",
            crate::workspace::timestamp()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let mut saved = Settings::default();
        saved.editor.size = 18.0;
        let backup = serde_json::to_vec(&saved).unwrap();
        std::fs::write(dir.join("settings.json.bak"), &backup).unwrap();
        std::fs::write(dir.join("settings.json"), b"broken").unwrap();
        let (restored, notice) = Settings::load(&dir);
        assert_eq!(restored.editor.size, 18.0);
        assert!(notice.is_some());
        restored.save(&dir).unwrap();
        assert_eq!(
            std::fs::read(dir.join("settings.json.bak")).unwrap(),
            backup
        );
    }

    #[test]
    fn newer_settings_with_unknown_fields_remain_untouched() {
        let dir = std::env::temp_dir().join(format!(
            "flowrite-settings-future-{}",
            crate::workspace::timestamp()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let bytes = b"{\"version\":999,\"editor\":\"future schema\"}";
        std::fs::write(dir.join("settings.json"), bytes).unwrap();
        let (settings, notice) = Settings::load(&dir);
        assert!(notice.unwrap().contains("newer"));
        assert!(settings.save(&dir).is_err());
        assert_eq!(std::fs::read(dir.join("settings.json")).unwrap(), bytes);
    }
}
