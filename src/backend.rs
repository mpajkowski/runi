use std::{convert::Infallible, str::FromStr};

pub mod eframe;
pub mod layer_shell;

#[derive(Debug, Default)]
pub enum UiBackend {
    LayerShell,
    Eframe,
    #[default]
    InferFromEnv,
}

impl FromStr for UiBackend {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "eframe" => Self::Eframe,
            "layer-shell" => Self::LayerShell,
            _ => Self::InferFromEnv,
        })
    }
}
