pub mod model;
mod ui;

use std::{env, path::PathBuf, str::FromStr, thread};

use anyhow::Result;

fn main() -> Result<()> {
    let apps_thread = thread::spawn(|| {
        let app_dirs = apps_dirs()?;

        let mut applications = app_dirs
            .iter()
            .filter_map(|path| std::fs::read_dir(path).ok())
            .flatten()
            .filter_map(|file| file.ok())
            .map(|file| file.path())
            .filter(|path| path.extension().and_then(|x| x.to_str()) == Some("desktop"))
            .filter_map(|file| model::Application::from_freedesktop_file(file).transpose())
            .collect::<Result<Vec<_>>>()?;

        applications.sort_by_cached_key(|x| x.name.clone());

        anyhow::Ok(applications)
    });

    ui::run_ui(apps_thread);

    Ok(())
}

fn apps_dirs() -> Result<Vec<PathBuf>> {
    let xdg_data_dirs = env::var("XDG_DATA_DIRS")?;

    let mut apps_dirs = xdg_data_dirs
        .split(":")
        .map(|d| PathBuf::from_str(d).map_err(|err| err.into()))
        .collect::<Result<Vec<_>>>()?;

    let xdg_data_home = match env::var("XDG_DATA_HOME") {
        Ok(p) => PathBuf::from_str(&p)?,
        Err(_) => {
            let home_dir = env::var("HOME")?;
            let mut xdg_data_home = PathBuf::from_str(&home_dir)?;
            xdg_data_home.push(".local/share");
            xdg_data_home
        }
    };

    apps_dirs.push(xdg_data_home);
    apps_dirs.iter_mut().for_each(|d| d.push("applications"));

    Ok(apps_dirs)
}
