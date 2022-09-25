pub mod model;

use anyhow::Result;

use gtk4::{prelude::*, Label, ListView, ScrolledWindow, SignalListItemFactory, SingleSelection};
use gtk4::{Application, ApplicationWindow};
use model::ApplicationObject;

fn main() -> Result<()> {
    let mut applications = std::fs::read_dir("/usr/share/applications")?
        .into_iter()
        .map(|file| file.unwrap().path())
        .filter(|path| path.extension().and_then(|x| x.to_str()) == Some("desktop"))
        .inspect(|path| println!("File: {}", path.display()))
        .map(model::Application::from_freedesktop_file)
        .collect::<Result<Vec<_>>>()?;

    applications.sort_by_cached_key(|x| x.name.clone());

    let applications = applications.leak();

    let application = Application::builder()
        .application_id("com.mpajkowski.GtkLauncher")
        .build();

    application.connect_activate(|gtk_app| build_ui(gtk_app, applications));
    application.run();

    Ok(())
}

fn build_ui(gtk_app: &Application, apps: &[model::Application]) {
    let apps = apps.iter().cloned().map(ApplicationObject::new);

    let mut model = gtk4::gio::ListStore::builder()
        .item_type(ApplicationObject::static_type())
        .build();

    model.extend(apps);

    let factory = SignalListItemFactory::new();
    factory.connect_setup(move |_, list_item| {
        let label = Label::new(None);
        list_item.set_child(Some(&label));
    });

    factory.connect_bind(move |_, list_item| {
        let entry = list_item
            .item()
            .expect("The item has to exist")
            .downcast::<ApplicationObject>()
            .expect("The item has to be ApplicationObject");

        let name = entry.property::<String>("name");

        let label = list_item.child().unwrap().downcast::<Label>().unwrap();
        label.set_label(&name);
    });

    let selection_model = SingleSelection::new(Some(&model));
    let list_view = ListView::new(Some(&selection_model), Some(&factory));

    let scrolled_window = ScrolledWindow::builder()
        .min_content_width(360)
        .min_content_height(1000)
        .child(&list_view)
        .build();

    let window = ApplicationWindow::builder()
        .application(gtk_app)
        .decorated(false)
        .resizable(false)
        .title("")
        .child(&scrolled_window)
        .build();

    window.show();
}
