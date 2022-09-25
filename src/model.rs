use std::cell::RefCell;
use std::path::Path;

use anyhow::{Context, Result};
use gtk4::glib::once_cell::sync::Lazy;
use gtk4::glib::{self, Object, ParamSpec, ParamSpecString, Value};
use gtk4::prelude::ToValue;
use gtk4::subclass::prelude::{ObjectImpl, ObjectSubclass};

#[derive(Debug, Default, Clone)]
pub struct Application {
    pub name: RefCell<String>,
    pub desc: RefCell<Option<String>>,
    pub exec: RefCell<String>,
    pub actions: RefCell<Vec<Action>>,
}

#[derive(Debug, Default, Clone)]
pub struct Action {
    pub name: RefCell<String>,
    pub exec: RefCell<String>,
}

impl Application {
    pub fn from_freedesktop_file(path: impl AsRef<Path>) -> Result<Self> {
        let entry = freedesktop_entry_parser::parse_entry(path)?;

        let mut name = None;
        let mut desc = None;
        let mut exec = None;
        let mut actions = vec![];

        for section in entry.sections() {
            let section_name = section
                .attr("Name")
                .map(|x| x.to_string())
                .context("Name not found")?;

            let section_exec = section
                .attr("Exec")
                .map(|x| x.to_string())
                .context("Exec not found")?;

            let section_desc = section.attr("Desc").map(|x| x.to_string());

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
                    name: RefCell::new(section_name),
                    exec: RefCell::new(section_exec),
                };
                actions.push(action);
            }
        }

        anyhow::ensure!(
            name.is_some() && exec.is_some(),
            "Section 'Desktop Entry' not found"
        );

        Ok(Application {
            name: RefCell::new(name.unwrap()),
            desc: RefCell::new(desc),
            exec: RefCell::new(exec.unwrap()),
            actions: RefCell::new(actions),
        })
    }
}

glib::wrapper! {
    pub struct ApplicationObject(ObjectSubclass<Application>);
}

impl ApplicationObject {
    pub fn new(application: Application) -> Self {
        Object::new(&[
            ("name", &*application.name.borrow()),
            ("desc", &*application.desc.borrow()),
            ("exec", &*application.exec.borrow()),
        ])
        .unwrap()
    }
}

#[glib::object_subclass]
impl ObjectSubclass for Application {
    const NAME: &'static str = "ModelApplication";
    type Type = ApplicationObject;
}

// TODO: handle actions
impl ObjectImpl for Application {
    fn properties() -> &'static [gtk4::glib::ParamSpec] {
        static PROPS: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
            vec![
                ParamSpecString::builder("name").build(),
                ParamSpecString::builder("desc").build(),
                ParamSpecString::builder("exec").build(),
            ]
        });

        PROPS.as_ref()
    }

    fn set_property(
        &self,
        _obj: &Self::Type,
        _id: usize,
        value: &gtk4::glib::Value,
        pspec: &gtk4::glib::ParamSpec,
    ) {
        match pspec.name() {
            "name" => {
                self.name.replace(value.get().unwrap());
            }
            "desc" => {
                self.desc.replace(value.get().unwrap());
            }
            "exec" => {
                self.exec.replace(value.get().unwrap());
            }
            _ => unimplemented!(),
        }
    }

    fn property(
        &self,
        _obj: &Self::Type,
        _id: usize,
        pspec: &gtk4::glib::ParamSpec,
    ) -> gtk4::glib::Value {
        match pspec.name() {
            "name" => self.name.borrow().to_value(),
            "desc" => self.desc.borrow().to_value(),
            "exec" => self.exec.borrow().to_value(),
            _ => unimplemented!(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_from_freedesktop_file() {
        let files = std::fs::read_dir("/usr/share/applications").unwrap();
        let apps = files
            .into_iter()
            .map(|file| file.unwrap().path())
            .filter(|path| path.extension().and_then(|x| x.to_str()) == Some("desktop"))
            .inspect(|path| println!("File: {}", path.display()))
            .map(Application::from_freedesktop_file)
            .collect::<Result<Vec<_>>>()
            .unwrap();

        println!("APPS: {apps:?}")
    }
}
