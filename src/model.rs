use std::{
    env,
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

#[derive(Debug, Default, Clone)]
pub struct Application {
    pub name: String,
    pub desc: Option<String>,
    pub exec: String,
    pub actions: Vec<Action>,
}

#[derive(Debug, Default, Clone)]
pub struct Action {
    pub name: String,
    pub exec: String,
}

impl Application {
    pub fn new(name: String, desc: Option<String>, exec: String, actions: Vec<Action>) -> Self {
        Self {
            name,
            desc,
            exec,
            actions,
        }
    }

    pub fn from_freedesktop_file(path: impl AsRef<Path>) -> Result<Option<Self>> {
        let entry = freedesktop_entry_parser::parse_entry(path)?;

        let mut name = None;
        let mut desc = None;
        let mut exec = None;
        let mut actions = vec![];

        for section in entry.sections() {
            if section.attr("Hidden") == Some("true") || section.attr("NoDisplay") == Some("true") {
                return Ok(None);
            }

            let section_name = section
                .attr("Name")
                .map(|x| x.to_string())
                .context("Name not found")?;

            let section_exec = section
                .attr("Exec")
                .map(|x| x.to_string())
                .context("Exec not found")?;

            let section_desc = section.attr("Comment").map(|x| x.to_string());

            if section.name() == "Desktop Entry" {
                anyhow::ensure!(
                    name.is_none() && exec.is_none(),
                    "Section 'Desktop Entry' defined twice",
                );

                name = Some(section_name);
                exec = Some(section_exec);
                desc = section_desc;
            } else if section.name().contains("Desktop Action") {
                let action = Action {
                    name: section_name,
                    exec: section_exec,
                };
                actions.push(action);
            }
        }

        anyhow::ensure!(
            name.is_some() && exec.is_some(),
            "Section 'Desktop Entry' not found"
        );

        Ok(Some(Self::new(name.unwrap(), desc, exec.unwrap(), actions)))
    }

    pub fn exec(&self) -> Result<()> {
        let mut exec_split = self.exec.split_whitespace();

        let command = exec_split.next().unwrap();
        let path = which(command).context("Failed to find executable")?;

        let args = exec_split
            .filter(|arg| {
                !matches!(
                    *arg,
                    "%f" | "%F"
                        | "%u"
                        | "%U"
                        | "%d"
                        | "%D"
                        | "%n"
                        | "%N"
                        | "%i"
                        | "%c"
                        | "%v"
                        | "%m"
                )
            })
            .collect::<Vec<_>>();

        std::process::Command::new(path).args(args).exec();

        unreachable!()
    }

    pub fn score(&self, filter: &str) -> f64 {
        let comp = |string| strsim::normalized_levenshtein(string, filter);

        comp(&self.name.to_lowercase())
            .max(comp(&self.exec.to_lowercase()) * 0.5)
            .max(self.desc.as_deref().map(comp).unwrap_or_default() * 0.25)
    }
}

fn which(executable: impl AsRef<Path>) -> Option<PathBuf> {
    let paths = env::var_os("PATH")?;
    let paths = env::split_paths(&paths);

    for path in paths {
        let absolute = path.join(&executable);
        if absolute.is_file() {
            return Some(absolute);
        }
    }

    None
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_from_freedesktop_file() {
        let application = Application::from_freedesktop_file(format!(
            "{}/test/Alacritty.desktop",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
        .unwrap();

        assert_eq!(application.name, "Alacritty");
        assert_eq!(application.exec, "alacritty");
        assert_eq!(
            application.desc,
            Some("A fast, cross-platform, OpenGL terminal emulator".to_string())
        );
        assert_eq!(application.actions.len(), 1);
        assert_eq!(application.actions[0].name, "New Terminal");
        assert_eq!(application.actions[0].exec, "alacritty");
    }
}
