use std::{
    fmt::Display,
    hash::{Hash, Hasher},
    os::unix::process::CommandExt,
    path::Path,
    str::FromStr,
};

use anyhow::{Context, Result};

#[derive(Debug, Clone, Eq)]
pub struct Application {
    pub name: String,
    pub desc: Option<String>,
    pub exec: Exec,
    pub path: Option<String>,
    pub actions: Vec<Action>,
    name_lower: String,
    exec_lower: Option<String>,
}

impl PartialEq for Application {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Hash for Application {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Action {
    pub name: String,
    pub exec: String,
}

impl Application {
    pub fn new(
        name: String,
        desc: Option<String>,
        exec: Exec,
        path: Option<String>,
        actions: Vec<Action>,
    ) -> Self {
        const EXEC_EXCLUDE: &[&str] = &["steam"];
        let name_lower = name.to_lowercase();
        let use_exec_for_search = !EXEC_EXCLUDE
            .iter()
            .any(|exclude| exec.cmd.contains(exclude));

        let exec_lower = use_exec_for_search.then_some(exec.cmd.to_lowercase());

        Self {
            name,
            desc,
            exec,
            path,
            actions,
            name_lower,
            exec_lower,
        }
    }

    pub fn from_freedesktop_file(path: impl AsRef<Path>) -> Result<Option<Self>> {
        let entry = freedesktop_entry_parser::parse_entry(path)?;

        let mut name = None;
        let mut desc = None;
        let mut exec = None;
        let mut path = None;
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
            let section_path = section.attr("Path").map(|x| x.to_string());

            if section.name() == "Desktop Entry" {
                anyhow::ensure!(
                    name.is_none() && exec.is_none(),
                    "Section 'Desktop Entry' defined twice",
                );

                name = Some(section_name);
                exec = Some(
                    section_exec
                        .parse()
                        .context("Failed to parse desktop file")?,
                );
                path = section_path;
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

        Ok(Some(Self::new(
            name.unwrap(),
            desc,
            exec.unwrap(),
            path,
            actions,
        )))
    }

    pub fn exec(&self) -> Result<()> {
        log::info!("Executing {}", self.exec);

        let Exec { cmd } = &self.exec;

        let mut command = std::process::Command::new("sh");
        command.arg("-c");
        command.arg(cmd);

        if let Some(path) = self.path.as_ref() {
            command.current_dir(path);
        }

        let _ = command.exec();

        Ok(())
    }

    pub fn score(&self, filter: &str) -> f64 {
        let filter = filter.to_lowercase();

        let score_str = |string: &str| {
            if string.contains(&filter) {
                return 1.0;
            }

            strsim::normalized_levenshtein(string, &filter)
        };

        score_str(&self.name_lower)
            .max(self.exec_lower.as_deref().map(score_str).unwrap_or(0.0) * 0.5)
    }
}

fn trim_arg(s: &str) -> &str {
    let mut out = s;
    [
        "%f", "%F", "%u", "%U", "%d", "%D", "%n", "%N", "%i", "%c", "%v", "%m",
    ]
    .iter()
    .for_each(|pat| out = out.trim_end_matches(pat));

    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exec {
    pub cmd: String,
}

impl FromStr for Exec {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let sanitized = s.trim();
        let sanitized = trim_arg(sanitized);

        Ok(Self {
            cmd: sanitized.to_string(),
        })
    }
}

impl Display for Exec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Command: {}", self.cmd)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_simple_freedesktop_file() {
        let application = Application::from_freedesktop_file(format!(
            "{}/test/Alacritty.desktop",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
        .unwrap();

        assert_eq!(application.name, "Alacritty");
        assert_eq!(
            application.exec,
            Exec {
                cmd: "alacritty".to_string(),
            }
        );
        assert_eq!(
            application.desc,
            Some("A fast, cross-platform, OpenGL terminal emulator".to_string())
        );
        assert_eq!(application.actions.len(), 1);
        assert_eq!(application.actions[0].name, "New Terminal");
        assert_eq!(application.actions[0].exec, "alacritty");
    }

    #[test]
    fn test_wineapp() {
        let application = Application::from_freedesktop_file(format!(
            "{}/test/Guitar Pro 7.desktop",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
        .unwrap();

        assert_eq!(application.name, "Guitar Pro 7");
        assert_eq!(
            application.exec,
            Exec {
                cmd: r#"env WINEPREFIX="/home/marcin/.wine" wine C:\\\\ProgramData\\\\Microsoft\\\\Windows\\\\Start\\ Menu\\\\Programs\\\\Guitar\\ Pro\\ 7\\\\Guitar\\ Pro\\ 7.lnk"#.to_owned(),
            }
        );
    }
}
