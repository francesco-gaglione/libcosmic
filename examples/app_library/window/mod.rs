mod imp;
use crate::app_group::AppGroup;
use crate::app_group::AppGroupData;
use crate::utils::data_path;
use crate::utils::set_group_scroll_policy;
use glib::FromVariant;
use glib::Variant;
use gtk4 as gtk;
use gtk4::Dialog;
use gtk4::Entry;
use gtk4::Label;
use std::fs::File;

use crate::grid_item::GridItem;
use glib::Object;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};
use gtk::{Application, SignalListItemFactory};

use libcosmic::x;

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Window {
    pub fn new(app: &Application) -> Self {
        //quit shortcut
        app.set_accels_for_action("win.quit", &["<primary>W", "Escape"]);
        //launch shortcuts
        for i in 1..10 {
            app.set_accels_for_action(&format!("win.launch{}", i), &[&format!("<primary>{}", i)]);
        }
        Object::new(&[("application", app)]).expect("Failed to create `Window`.")
    }

    fn _app_model(&self) -> &gio::ListStore {
        // Get state
        let imp = imp::Window::from_instance(self);
        imp.app_model.get().expect("Could not get model")
    }

    fn group_model(&self) -> &gio::ListStore {
        // Get state
        let imp = imp::Window::from_instance(self);
        imp.group_model.get().expect("Could not get model")
    }

    fn setup_model(&self) {
        // Create new model
        let app_model = gio::ListStore::new(gio::DesktopAppInfo::static_type());
        // Get state and set model
        let imp = imp::Window::from_instance(self);

        // A sorter used to sort AppInfo in the model by their name
        xdg::BaseDirectories::new()
            .expect("could not access XDG Base directory")
            .get_data_dirs()
            .iter_mut()
            .for_each(|xdg_data_path| {
                xdg_data_path.push("applications");
                dbg!(&xdg_data_path);
                if let Ok(dir_iter) = std::fs::read_dir(xdg_data_path) {
                    dir_iter.for_each(|dir_entry| {
                        if let Ok(dir_entry) = dir_entry {
                            if let Some(path) = dir_entry.path().file_name() {
                                if let Some(path) = path.to_str() {
                                    if let Some(app_info) = gio::DesktopAppInfo::new(path) {
                                        if app_info.should_show() {
                                            app_model.append(&app_info)
                                        } else {
                                            println!("Ignoring {}", path);
                                        }
                                    } else {
                                        println!("error loading {}", path);
                                    }
                                }
                            }
                        }
                    })
                }
            });
        imp.app_model
            .set(app_model.clone())
            .expect("Could not set model");

        let sorter = gtk::CustomSorter::new(move |obj1, obj2| {
            let app_info1 = obj1.downcast_ref::<gio::DesktopAppInfo>().unwrap();
            let app_info2 = obj2.downcast_ref::<gio::DesktopAppInfo>().unwrap();

            app_info1
                .name()
                .to_lowercase()
                .cmp(&app_info2.name().to_lowercase())
                .into()
        });
        let filter = gtk::CustomFilter::new(|_obj| true);
        let search_filter_model =
            gtk::FilterListModel::new(Some(&app_model), Some(filter).as_ref());
        let filter = gtk::CustomFilter::new(|_obj| true);
        let group_filter_model =
            gtk::FilterListModel::new(Some(&search_filter_model), Some(filter).as_ref());
        let sorted_model = gtk::SortListModel::new(Some(&group_filter_model), Some(&sorter));
        let selection_model = gtk::SingleSelection::builder()
            .model(&sorted_model)
            .autoselect(false)
            .can_unselect(true)
            .selected(gtk4::INVALID_LIST_POSITION)
            .build();

        // Wrap model with selection and pass it to the list view
        imp.app_grid_view.set_model(Some(&selection_model));
        selection_model.unselect_all();

        let group_model = gio::ListStore::new(AppGroup::static_type());
        imp.group_model
            .set(group_model.clone())
            .expect("Could not set group model");
        vec![
            AppGroup::new(AppGroupData {
                id: 0,
                name: "Library Home".to_string(),
                icon: "user-home".to_string(),
                mutable: false,
                app_names: Vec::new(),
                category: "".to_string(),
            }),
            AppGroup::new(AppGroupData {
                id: 0,
                name: "System".to_string(),
                icon: "folder".to_string(),
                mutable: false,
                app_names: Vec::new(),
                category: "System".to_string(),
            }),
            AppGroup::new(AppGroupData {
                id: 0,
                name: "Utilities".to_string(),
                icon: "folder".to_string(),
                mutable: false,
                app_names: Vec::new(),
                category: "Utility".to_string(),
            }),
            // Example of group with app name
            // AppGroup::new(AppGroupData {
            //     id: 0,
            //     name: "Custom Web".to_string(),
            //     icon: "folder".to_string(),
            //     mutable: true,
            //     app_names: vec!["Firefox Web Browser".to_string()],
            //     category: "".to_string(),
            // }),
            AppGroup::new(AppGroupData {
                id: 0,
                name: "New Group".to_string(),
                icon: "folder-new".to_string(),
                mutable: true,
                app_names: vec![],
                category: "".to_string(),
            }),
        ]
        .iter()
        .for_each(|group| {
            group_model.append(group);
        });
        let group_selection = gtk4::SingleSelection::new(Some(&group_model));
        imp.group_grid_view.set_model(Some(&group_selection));
    }

    fn setup_callbacks(&self) {
        // Get state
        let imp = imp::Window::from_instance(self);
        let window = self.clone().upcast::<gtk::Window>();
        let app_grid_view = &imp.app_grid_view;
        let group_grid_view = &imp.group_grid_view;
        let app_selection_model = app_grid_view
            .model()
            .expect("List view missing selection model")
            .downcast::<gtk::SingleSelection>()
            .expect("could not downcast listview model to single selection model");
        let app_sorted_model = app_selection_model
            .model()
            .downcast::<gtk::SortListModel>()
            .expect("sorted list model could not be downcast");
        let app_group_filter_model = app_sorted_model
            .model()
            .expect("missing model for sort list model.")
            .downcast::<gtk::FilterListModel>()
            .expect("could not downcast sort list model to filter list model");
        let app_filter_model = app_group_filter_model
            .model()
            .expect("missing model for sort list model.")
            .downcast::<gtk::FilterListModel>()
            .expect("could not downcast sort list model to filter list model");
        let group_selection_model = group_grid_view
            .model()
            .expect("List view missing selection model")
            .downcast::<gtk::SingleSelection>()
            .expect("could not downcast listview model to single selection model");

        let entry = &imp.entry;
        let scroll_window = &imp.group_scroll_window.get();

        // dynamically set scroll method bc of buggy gtk scroll behavior
        self.group_model().connect_items_changed(
            glib::clone!(@weak scroll_window => move |scroll_list_model, _i, _rmv_cnt, _add_cnt| {
                set_group_scroll_policy(&scroll_window, scroll_list_model.n_items());
            }),
        );
        app_selection_model.connect_selected_notify(glib::clone!(@weak window => move |model| {
             // on activation change the group filter model to use the app names, and category
            let position = model.selected();
            println!("selected app {}", position);
            // Launch the application when an item of the list is activated
            if let Some(item) = model.item(position) {
                let app_info = item.downcast::<gio::DesktopAppInfo>().unwrap();
                let context = window.display().app_launch_context();
                if let Err(err) = app_info.launch(&[], Some(&context)) {
                    gtk::MessageDialog::builder()
                        .text(&format!("Failed to start {}", app_info.name()))
                        .secondary_text(&err.to_string())
                        .message_type(gtk::MessageType::Error)
                        .modal(true)
                        .transient_for(&window)
                        .build()
                        .show();
                }

            }
        }));

        group_selection_model.connect_selected_notify(glib::clone!(@weak app_filter_model, @weak window => move |group_selection_model| {
            // on activation change the group filter model to use the app names, and category
            let position = group_selection_model.selected();
            println!("grid view activated. {}", position);
        // group_grid_view.connect_activate(glib::clone!(@weak app_filter_model, @weak window => move |grid_view, position| {
            let group_model = group_selection_model
                .model()
                .downcast::<gio::ListStore>()
                .expect("could not downcast app group view selection model to list store model");

           // if last item in the model, don't change filter, instead show dialog for adding new group!
            if position == group_model.n_items() - 1 {
                let dialog_entry = Entry::new();
                let label = Label::new(Some("Name"));
                label.set_justify(gtk4::Justification::Left);
                label.set_xalign(0.0);
                let vbox = gtk4::Box::builder()
                    .spacing(12)
                    .hexpand(true)
                    .orientation(gtk4::Orientation::Vertical)
                    .margin_top(12)
                    .margin_bottom(12)
                    .margin_end(12)
                    .margin_start(12)
                    .build();
                vbox.append(&label);
                vbox.append(&dialog_entry);

                let dialog = Dialog::builder()
                    .modal(true)
                    .resizable(false)
                    .use_header_bar(true.into())
                    .destroy_with_parent(true)
                    .transient_for(&window)
                    .title("New App Group")
                    .child(&vbox)
                    .build();
                let app = window
                    .application()
                    .expect("could not get application from window");

                dialog.set_application(Some(&app));
                dialog.add_buttons(&[
                    ("Apply", gtk4::ResponseType::Apply),
                    ("Cancel", gtk4::ResponseType::Cancel),
                ]);

               dialog.connect_response(
                    glib::clone!(@weak dialog_entry, @weak group_selection_model, @weak group_model => move |dialog, response_type| {
                        println!("dialog should be closing...");
                        let name = dialog_entry.text().to_string();
                        if response_type == gtk4::ResponseType::Apply && name != "" {
                            let new_app_group = AppGroup::new(AppGroupData {
                                id: 0,
                                name: name,
                                icon: "folder".to_string(),
                                mutable: true,
                                app_names: vec![],
                                category: "".to_string(),
                            });
                            group_model.insert(group_model.n_items() - 1, &new_app_group);
                            group_selection_model.set_selected(position - 1);
                        } else {
                            group_selection_model.set_selected(0);
                        }
                        dialog.emit_close();
                    }),
                );
                dialog.connect_is_active_notify(move |win| {
                    let app = win
                        .application()
                        .expect("could not get application from window");
                    let active_window = app
                        .active_window()
                        .expect("no active window available, closing app library.");
                    dbg!(&active_window);
                    if win == &active_window && !win.is_active() {
                        println!("no focus");
                        // close top level window
                        window.close();
                    }
                });
                dialog.show();
                return;
            };
            // update the application filter
            let app_info = group_model
                .item(position)
                .unwrap()
                .downcast::<AppGroup>()
                .unwrap();
            let category =
            if let Ok(category_prop) = app_info.property("category") {
                category_prop.get::<String>().unwrap_or("".to_string()).to_lowercase()
            } else {
                "".to_string()
            };

            let app_names =
                if let Ok(app_names_prop) = app_info.property("appnames") {
                    <Vec<String>>::from_variant(&app_names_prop.get::<Variant>().expect("appnames nneds to be a variant.")).unwrap_or_default()
            } else {
                vec![]
            };
            dbg!(&app_names);
            let new_filter: gtk::CustomFilter = gtk::CustomFilter::new(move |obj| {
                let app = obj
                    .downcast_ref::<gio::DesktopAppInfo>()
                    .expect("The Object needs to be of type AppInfo");
                if app_names.len() > 0 {
                    return app_names.contains(&String::from(app.name().as_str()));
                }
                match app.categories() {
                    Some(categories) => categories.to_string().to_lowercase().contains(&category),
                    None => false,
                }
            });
            app_group_filter_model.set_filter(Some(new_filter).as_ref());
        }));

        entry.connect_changed(
            glib::clone!(@weak app_filter_model, @weak app_sorted_model => move |search: &gtk::SearchEntry| {
                let search_text = search.text().to_string().to_lowercase();
                let new_filter: gtk::CustomFilter = gtk::CustomFilter::new(move |obj| {
                    let search_res = obj.downcast_ref::<gio::DesktopAppInfo>()
                        .expect("The Object needs to be of type AppInfo");
                    search_res.name().to_string().to_lowercase().contains(&search_text)
                });
                let search_text = search.text().to_string().to_lowercase();
                let new_sorter: gtk::CustomSorter = gtk::CustomSorter::new(move |obj1, obj2| {
                    let app_info1 = obj1.downcast_ref::<gio::DesktopAppInfo>().unwrap();
                    let app_info2 = obj2.downcast_ref::<gio::DesktopAppInfo>().unwrap();
                    if search_text == "" { 
                        return app_info1
                            .name()
                            .to_lowercase()
                            .cmp(&app_info2.name().to_lowercase())
                            .into();
                    }

                    let i_1 = app_info1.name().to_lowercase().find(&search_text);
                    let i_2 = app_info2.name().to_lowercase().find(&search_text);
                    match (i_1, i_2) {
                        (Some(i_1), Some(i_2)) => i_1.cmp(&i_2).into(),
                        (Some(_), None) => std::cmp::Ordering::Less.into(),
                        (None, Some(_)) => std::cmp::Ordering::Greater.into(),
                        _ => app_info1
                            .name()
                            .to_lowercase()
                            .cmp(&app_info2.name().to_lowercase())
                            .into()
                    }
                });

                app_filter_model.set_filter(Some(new_filter).as_ref());
                app_sorted_model.set_sorter(Some(new_sorter).as_ref());
            }),
        );

        window.connect_realize(move |window| {
            if let Some((display, surface)) = x::get_window_x11(window) {
                unsafe {
                    x::change_property(
                        &display,
                        &surface,
                        "_NET_WM_WINDOW_TYPE",
                        x::PropMode::Replace,
                        &[x::Atom::new(&display, "_NET_WM_WINDOW_TYPE_DIALOG").unwrap()],
                    );
                }
            } else {
                println!("failed to get X11 window");
            }
        });

        let action_quit = gio::SimpleAction::new("quit", None);
        action_quit.connect_activate(glib::clone!(@weak window => move |_, _| {
            window.close();
        }));
        self.add_action(&action_quit);
        window.connect_is_active_notify(move |win| {
            let app = win
                .application()
                .expect("could not get application from window");
            let active_window = app
                .active_window()
                .expect("no active window available, closing app library.");
            dbg!(&active_window);
            if win == &active_window && !win.is_active() {
                win.close();
            }
        });
    }

    fn setup_factory(&self) {
        let app_factory = SignalListItemFactory::new();
        app_factory.connect_setup(move |_factory, item| {
            let row = GridItem::new();
            item.set_child(Some(&row));
        });

        let imp = imp::Window::from_instance(self);
        // the bind stage is used for "binding" the data to the created widgets on the "setup" stage
        let app_grid_view = &imp.app_grid_view.get();
        app_factory.connect_bind(
            glib::clone!(@weak app_grid_view => move |_factory, grid_item| {
                let app_info = grid_item
                    .item()
                    .unwrap()
                    .downcast::<gio::DesktopAppInfo>()
                    .unwrap();

                let child = grid_item.child().unwrap().downcast::<GridItem>().unwrap();
                child.set_app_info(&app_info);
            }),
        );
        // Set the factory of the list view
        app_grid_view.set_factory(Some(&app_factory));

        let group_factory = SignalListItemFactory::new();
        group_factory.connect_setup(move |_factory, item| {
            let row = GridItem::new();
            item.set_child(Some(&row));
        });

        // the bind stage is used for "binding" the data to the created widgets on the "setup" stage
        group_factory.connect_bind(move |_factory, grid_item| {
            let group_info = grid_item.item().unwrap().downcast::<AppGroup>().unwrap();

            let child = grid_item.child().unwrap().downcast::<GridItem>().unwrap();
            child.set_group_info(group_info);
        });
        // Set the factory of the list view
        imp.group_grid_view.set_factory(Some(&group_factory));
    }

    fn restore_data(&self) {
        if let Ok(file) = File::open(data_path()) {
            // Deserialize data from file to vector
            let backup_data: Vec<AppGroupData> =
                serde_json::from_reader(file).expect("Could not get backup data from json file.");

            let app_group_objects: Vec<Object> = backup_data
                .into_iter()
                .map(|data| AppGroup::new(data).upcast::<Object>())
                .collect();
            let scroll_window = &imp::Window::from_instance(self).group_scroll_window;

            // Insert restored objects into model
            self.group_model().splice(3, 0, &app_group_objects);
            set_group_scroll_policy(&scroll_window, self.group_model().n_items());
        } else {
            println!("Backup file does not exist yet {:?}", data_path());
        }
    }
}
