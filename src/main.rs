mod flock;
mod loader;
mod ui;

pub mod config;
pub mod model;

use std::thread;

use anyhow::Result;

use crate::loader::load_apps;
pub use flock::Lock;

fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();

    log::info!("init");

    let Some(flock) = flock::Lock::obtain() else {
        return Ok(());
    };

    let apps_thread = thread::spawn(load_apps);

    ui::run_ui(apps_thread, flock);

    Ok(())
}
