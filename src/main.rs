mod backend;
mod flock;
mod loader;
mod ui;

pub mod config;
pub mod model;

use std::thread;

use anyhow::Result;

use crate::{backend::UiBackend, loader::load_apps};
pub use flock::Lock;

fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();

    let backend: UiBackend = std::env::args()
        .nth(1)
        .map(|value| value.parse().unwrap())
        .unwrap_or_default();

    log::info!("init, selected backend: {backend:?}");

    let Some(flock) = flock::Lock::obtain() else {
        log::info!("another instance detected; exiting");
        return Ok(());
    };

    let apps_thread = thread::spawn(load_apps);

    ui::run_ui(apps_thread, backend, flock);

    Ok(())
}
