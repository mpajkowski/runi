pub mod model;
mod ui;

use walkdir::WalkDir;

use std::{borrow::Cow, env, path::PathBuf, thread};

use anyhow::Result;

fn main() -> Result<()> {
    let apps_thread = thread::spawn(|| {
        let app_dirs = apps_dirs()?;

        let mut applications = app_dirs
            .iter()
            .flat_map(|path| WalkDir::new(path).into_iter())
            .filter_map(|file| file.ok())
            .filter(|file| file.path().extension().and_then(|x| x.to_str()) == Some("desktop"))
            .filter_map(|file| {
                let path = file.path();
                let app = model::Application::from_freedesktop_file(path);

                match app {
                    Ok(app) => app,
                    Err(err) => {
                        eprintln!("Failed to parse path: {:?}, err: {err}", path.display());
                        None
                    }
                }
            })
            .collect::<Vec<_>>();

        applications.sort_by_cached_key(|x| x.name.clone());

        anyhow::Ok(applications)
    });

    ui::run_ui(apps_thread);

    Ok(())
}

fn apps_dirs() -> Result<Vec<PathBuf>> {
    let xdg_data_dirs = env::var("XDG_DATA_DIRS")
        .map(Cow::Owned)
        .unwrap_or_else(|_| Cow::Borrowed("/usr/local/share/:/usr/share/"));

    let apps_dirs = xdg_data_dirs
        .split(':')
        .map(|d| anyhow::Ok(PathBuf::from(d)));

    let xdg_data_home = env::var("XDG_DATA_HOME")
        .map(|x| anyhow::Ok(PathBuf::from(x)))
        .unwrap_or_else(|_| {
            let home_dir = env::var("HOME")?;
            let mut xdg_data_home = PathBuf::from(&home_dir);
            xdg_data_home.push(".local/share");

            anyhow::Ok(xdg_data_home)
        });

    apps_dirs
        .chain([xdg_data_home])
        .map(|res| res.map(|dir| dir.join("applications")))
        .collect::<Result<Vec<_>>>()
}
