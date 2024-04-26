use std::{
    borrow::Cow,
    collections::HashMap,
    io::{BufReader, Read},
    path::{Path, PathBuf},
};

use egui::Color32;
use serde::{Deserialize, Deserializer};

use crate::model::Exec;

pub const SELECTION_COLOR: Color32 = Color32::DARK_RED;
pub const BACKGROUND_COLOR: Color32 = Color32::TRANSPARENT;

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    #[serde(rename = "patch")]
    pub patches: HashMap<PathBuf, Patch>,
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let file = std::fs::File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut buf = String::new();
        reader.read_to_string(&mut buf)?;

        let cfg = toml::from_str(&buf)?;

        Ok(cfg)
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Patch {
    #[serde(deserialize_with = "deserialize_exec")]
    pub exec: Exec,
}

fn deserialize_exec<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Exec, D::Error> {
    let s = Cow::<'static, str>::deserialize(deserializer)?;
    s.parse().map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_config_file() {
        let config =
            Config::load(format!("{}/test/config.toml", env!("CARGO_MANIFEST_DIR"))).unwrap();

        let (path, patch) = config.patches.into_iter().next().unwrap();
        assert_eq!(
            path,
            PathBuf::from("/usr/share/applications/signal-desktop.desktop")
        );
        assert_eq!(
            patch,
            Patch {
                exec: Exec {
                    cmd: "alacritty".to_owned(),
                    args: vec!["-v".to_owned()]
                }
            }
        );
    }
}
