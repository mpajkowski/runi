use std::{collections::HashSet, env, path::PathBuf};
use walkdir::WalkDir;

use crate::model::Application;

pub fn load_apps() -> Vec<Application> {
    let AppDirs { system, user } = app_dirs();
    let mut set: HashSet<Application> = HashSet::new();

    for dir in system {
        let apps = process_dir(dir);

        set.extend(apps);
    }

    if let Some(user) = user {
        let user_apps = process_dir(user);

        for app in user_apps {
            if let Some(system) = set.replace(app) {
                log::info!("overriden {}", system.name);
            }
        }
    }

    let mut apps: Vec<_> = set.into_iter().collect();
    apps.sort_unstable_by(|l, r| l.name.cmp(&r.name));

    log::info!("loaded {} apps", apps.len());

    apps
}

fn process_dir(dir: PathBuf) -> Vec<Application> {
    let dir = dir.join("applications");

    log::info!("processing dir: {}", dir.display());

    let walkdir = WalkDir::new(dir);

    let mut apps = vec![];
    for file in walkdir.into_iter() {
        let file = match file {
            Ok(file) if file.path().extension().and_then(|x| x.to_str()) == Some("desktop") => file,
            _ => continue,
        };

        let file = file.path();

        log::info!("processing file: {}", file.display());

        let app = match Application::from_freedesktop_file(file) {
            Ok(Some(app)) => app,
            Ok(None) => continue,
            Err(err) => {
                log::warn!("Failed to parse path: {:?}, err: {err}", file.display());
                continue;
            }
        };

        apps.push(app);
    }

    apps
}

struct AppDirs {
    system: Vec<PathBuf>,
    user: Option<PathBuf>,
}

fn app_dirs() -> AppDirs {
    let system = if let Ok(xdg_data_dirs) = env::var("XDG_DATA_DIRS") {
        xdg_data_dirs.split(':').map(PathBuf::from).collect()
    } else {
        vec![
            PathBuf::from("/usr/local/share"),
            PathBuf::from("/usr/share"),
        ]
    };

    let user = if let Ok(xdg_data_home) = env::var("XDG_DATA_HOME") {
        Some(PathBuf::from(xdg_data_home))
    } else if let Ok(home_dir) = env::var("HOME") {
        let mut dir = PathBuf::from(home_dir);
        dir.push(".local/share");
        Some(dir)
    } else {
        None
    };

    AppDirs { system, user }
}
