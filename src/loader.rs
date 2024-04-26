use std::{collections::HashSet, env, path::PathBuf, time::Instant};
use walkdir::WalkDir;

use crate::{config::Config, model::Application};

pub fn load_apps() -> Vec<Application> {
    let timer = Instant::now();
    let AppDirs { system, user, home } = app_dirs();

    let mut config = home
        .and_then(|mut cfg_dir| {
            cfg_dir.push(".config");
            cfg_dir.push("runi");
            cfg_dir.push("config.toml");

            Config::load(&cfg_dir)
                .map_err(|err| {
                    log::warn!(
                        "failed to load config from path {}: {err}",
                        cfg_dir.display()
                    )
                })
                .ok()
        })
        .unwrap_or_default();

    let mut set: HashSet<Application> = HashSet::new();

    for dir in system {
        let apps = process_dir(dir, &mut config);

        set.extend(apps);
    }

    if let Some(user) = user {
        let user_apps = process_dir(user, &mut config);

        for app in user_apps {
            if let Some(system) = set.replace(app) {
                log::info!("overriden {}", system.name);
            }
        }
    }

    let mut apps: Vec<_> = set.into_iter().collect();
    apps.sort_unstable_by(|l, r| l.name.cmp(&r.name));

    log::info!(
        "loaded {} apps in {}ms",
        apps.len(),
        timer.elapsed().as_millis()
    );

    apps
}

fn process_dir(dir: PathBuf, cfg: &mut Config) -> Vec<Application> {
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

        let mut app = match Application::from_freedesktop_file(file) {
            Ok(Some(app)) => app,
            Ok(None) => continue,
            Err(err) => {
                log::warn!("Failed to parse path: {:?}, err: {err}", file.display());
                continue;
            }
        };

        if let Some(patch) = cfg.patches.remove(file) {
            app.exec = patch.exec;
        }

        apps.push(app);
    }

    apps
}

struct AppDirs {
    system: Vec<PathBuf>,
    user: Option<PathBuf>,
    home: Option<PathBuf>,
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

    let mut home_dir = None;

    let user = if let Ok(xdg_data_home) = env::var("XDG_DATA_HOME") {
        Some(PathBuf::from(xdg_data_home))
    } else if let Ok(home) = env::var("HOME") {
        let mut dir = PathBuf::from(home);
        home_dir = Some(dir.clone());
        dir.push(".local/share");
        Some(dir)
    } else {
        None
    };

    AppDirs {
        system,
        user,
        home: home_dir,
    }
}
