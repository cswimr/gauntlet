use std::collections::HashMap;
use std::thread;

use tokio::task::LocalSet;

use crate::gtk::{PluginContainerContainer, PluginEventSenderContainer, PluginUiContext, PluginUiData};
use crate::gtk::gtk_side::start_request_receiver_loop;
use crate::gtk::gui::{AppInput, AppModel};
use crate::plugins::PluginManager;
use crate::react_side::{PluginReactData, run_react};
use crate::search::{SearchIndex, SearchItem};

// enum StartupStatus {
//     Server,
//     Agent,
//     Error(zbus::Error),
// }

// struct Greeter;
//
// #[zbus::dbus_interface(name = "org.placeholdername.PlaceHolderName")]
// impl Greeter {
//
// }

pub fn run_server() {
    // let status = relm4::gtk::glib::MainContext::default().block_on(async {
    //     let _conn = zbus::ConnectionBuilder::session()?
    //         .name("org.placeholdername.PlaceHolderName")?
    //         .serve_at("/org/placeholdername/PlaceHolderName", Greeter)?
    //         .build()
    //         .await?;
    //
    //
    //     let conn = zbus::Connection::session().await?;
    //
    //     match conn.request_name("org.placeholdername.placeholdername") {
    //         Ok(()) => StartupStatus::Server,
    //         Err(zbus::Error::NameTaken) => StartupStatus::Agent,
    //         Err(error) => StartupStatus::Error(error)
    //     }
    // });
    //
    // match status {
    //     StartupStatus::Server => {}
    //     StartupStatus::Agent => {}
    //     StartupStatus::Error(_) => {}
    // }

    let mut plugin_manager = PluginManager::create();
    let mut search_index = SearchIndex::create_index().unwrap();

    let search_items: Vec<_> = plugin_manager.plugins()
        .iter()
        .flat_map(|plugin| {
            plugin.entrypoints()
                .iter()
                .map(|entrypoint| {
                    SearchItem {
                        entrypoint_name: entrypoint.name().to_owned(),
                        entrypoint_id: entrypoint.id().to_owned(),
                        plugin_name: plugin.name().to_owned(),
                        plugin_id: plugin.id().to_owned(),
                    }
                })
        })
        .collect();

    search_index.add_entries(search_items).unwrap();

    let (react_contexts, ui_contexts) = plugin_manager.create_all_contexts();

    spawn_gtk_thread(ui_contexts, plugin_manager, search_index);

    run_react_loops(react_contexts);
}

fn spawn_gtk_thread(ui_data: Vec<PluginUiData>, plugin_manager: PluginManager, search_index: SearchIndex) {
    let handle = move || {
        let (contexts, event_senders): (Vec<_>, Vec<_>) = ui_data.into_iter()
            .map(|ui_data| {
                let context = (ui_data.plugin.clone(), ui_data.request_receiver);
                let event_sender = (ui_data.plugin.id().to_owned(), (ui_data.event_sender, ui_data.event_waker));
                (context, event_sender)
            })
            .unzip();

        let ui_contexts = contexts.into_iter()
            .map(|(plugin, receiver)| PluginUiContext::new(plugin, receiver))
            .collect::<Vec<_>>();

        let event_senders = event_senders.into_iter()
            .collect::<HashMap<_, _>>();

        let container_container = PluginContainerContainer::new();
        let event_senders_container = PluginEventSenderContainer::new(event_senders);

        start_request_receiver_loop(ui_contexts, container_container.clone(), event_senders_container.clone());

        let input = AppInput {
            search: search_index.create_handle(),
            plugin_manager,
            container_container,
            event_senders_container,
        };

        relm4::RelmApp::from_app(relm4::gtk::Application::builder().build())
            .run::<AppModel>(input);
    };

    thread::Builder::new()
        .name("gtk-thread".into())
        .spawn(handle)
        .expect("failed to spawn thread");
}


fn run_react_loops(react_contexts: Vec<PluginReactData>) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let local_set = LocalSet::new();

    local_set.block_on(&runtime, async {
        let mut join_set = tokio::task::JoinSet::new();
        for react_context in react_contexts {
            join_set.spawn_local(tokio::task::unconstrained(async {
                run_react(react_context).await
            }));
        }
        while let Some(_) = join_set.join_next().await {}
    });
}
