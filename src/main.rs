pub mod model;
mod ui;

use std::thread;

use anyhow::Result;

fn main() -> Result<()> {
    let apps_thread = thread::spawn(|| {
        let user_home_app_dir = format!("{}/.local/share/applications", env!("HOME"));
        let app_dirs = vec![
            "/usr/share/applications",
            "/usr/local/share/applications",
            &user_home_app_dir,
        ];

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
