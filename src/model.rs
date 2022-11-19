use std::{
    borrow::Cow,
    env,
    fmt::Display,
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    str::{FromStr, SplitWhitespace},
};

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Application {
    pub name: String,
    pub desc: Option<String>,
    pub exec: Exec,
    pub path: Option<String>,
    pub actions: Vec<Action>,
}

#[derive(Debug, Default, Clone)]
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
        Self {
            name,
            desc,
            exec,
            path,
            actions,
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
        println!("Executing {}", self.exec);

        let Exec { cmd, args } = &self.exec;

        let path = which(cmd).context("Failed to find executable")?;

        let mut command = std::process::Command::new(path);
        command.args(args);

        if let Some(path) = self.path.as_ref() {
            command.current_dir(path);
        }

        let _ = command.exec();

        Ok(())
    }

    pub fn score(&self, filter: &str) -> f64 {
        let comp = |string| strsim::normalized_levenshtein(string, filter);

        comp(&self.name.to_lowercase())
            .max(comp(&self.exec.cmd) * 0.5)
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

fn skip_arg(arg: impl AsRef<str>) -> bool {
    matches!(
        arg.as_ref(),
        "%f" | "%F" | "%u" | "%U" | "%d" | "%D" | "%n" | "%N" | "%i" | "%c" | "%v" | "%m"
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exec {
    pub cmd: String,
    pub args: Vec<String>,
}

impl FromStr for Exec {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let sanitized = s.replace(r"\\\\", r"\").replace('"', "");

        let mut iterator = ExecIterator::new(&sanitized);

        let cmd = iterator.next().context("Command not found")?;
        let args = iterator
            .filter(|s| !skip_arg(s))
            .map(Cow::into_owned)
            .collect();

        Ok(Self {
            cmd: cmd.to_string(),
            args,
        })
    }
}

impl Display for Exec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Command: {}, Args: [{}]", self.cmd, self.args.join(", "))
    }
}

struct ExecIterator<'a> {
    split: SplitWhitespace<'a>,
}

impl<'a> ExecIterator<'a> {
    fn new(s: &'a str) -> Self {
        Self {
            split: s.split_whitespace(),
        }
    }
}

impl<'a> Iterator for ExecIterator<'a> {
    type Item = Cow<'a, str>;

    fn next(&mut self) -> Option<Self::Item> {
        use std::fmt::Write;

        let val = self.split.next()?;

        if !val.ends_with(r"\\") {
            return Some(Cow::Borrowed(val));
        }

        let mut val = val.trim_end_matches(r"\\").to_string();

        for next in self.split.by_ref() {
            let continue_ = next.ends_with(r"\\");

            write!(&mut val, " {}", next.trim_end_matches(r"\\")).ok();

            if !continue_ {
                return Some(Cow::Owned(val));
            }
        }

        None
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
                args: vec![]
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
                cmd: "env".to_string(),
                args: vec![
                    "WINEPREFIX=/home/marcin/.wine".to_string(),
                    "wine".to_string(),
                    r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs\Guitar Pro 7\Guitar Pro 7.lnk".to_string(),
                ],
            }
        );
    }
}
