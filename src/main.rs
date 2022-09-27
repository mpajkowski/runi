pub mod model;
mod ui;

use std::thread;

use anyhow::Result;

fn main() -> Result<()> {
    let apps_thread = thread::spawn(|| {
        let mut applications = std::fs::read_dir("/usr/share/applications")?
            .into_iter()
            .map(|file| file.unwrap().path())
            .filter(|path| path.extension().and_then(|x| x.to_str()) == Some("desktop"))
            .filter_map(|file| model::Application::from_freedesktop_file(file).transpose())
            .collect::<Result<Vec<_>>>()?;

        applications.sort_by_cached_key(|x| x.name.clone());

        anyhow::Ok(applications)
    });

    ui::run_ui(apps_thread);

    Ok(())
}
