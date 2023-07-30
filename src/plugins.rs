use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use deno_core::anyhow::Context;
use deno_core::futures::task::AtomicWaker;
use deno_core::serde_json;
use directories::ProjectDirs;
use serde::Deserialize;
use crate::react_side::{PluginReactContext, UiEvent, UiRequest};
use crate::PluginUiContext;

#[derive(Clone)]
pub struct PluginManager {
    inner: Rc<RefCell<PluginManagerInner>>,
}

pub struct PluginManagerInner {
    plugins: Vec<Plugin>,
    ui_contexts: HashMap<String, PluginUiContext>
}

impl PluginManager {

    pub fn create() -> Self {
        let plugins = PluginLoader.load_plugins();

        Self {
            inner: Rc::new(RefCell::new(PluginManagerInner {
                plugins,
                ui_contexts: HashMap::new(),
            }))
        }
    }

    pub fn get_ui_context(&mut self, plugin_id: &str) -> Option<PluginUiContext> {
        self.inner
            .borrow_mut()
            .ui_contexts
            .get_mut(plugin_id)
            .map(|context| context.clone())
    }

    pub fn create_all_contexts(&mut self) -> (Vec<PluginReactContext>, Vec<PluginUiContext>) {
        let (react_contexts, ui_contexts): (Vec<_>, Vec<_>) = self.inner
            .borrow()
            .plugins
            .iter()
            .map(|plugin| self.create_contexts_for_plugin(plugin.clone()))
            .unzip();

        self.inner.borrow_mut().ui_contexts = ui_contexts.iter()
            .map(|context| (context.plugin.id().clone(), context.clone()))
            .collect::<HashMap<_, _>>();

        (react_contexts, ui_contexts)
    }

    fn create_contexts_for_plugin(&self, plugin: Plugin) -> (PluginReactContext, PluginUiContext) {
        let (react_request_sender, react_request_receiver) = tokio::sync::mpsc::unbounded_channel::<UiRequest>();
        let react_request_receiver = Rc::new(RefCell::new(react_request_receiver));

        let (react_event_sender, react_event_receiver) = std::sync::mpsc::channel::<UiEvent>();
        let event_waker = Arc::new(AtomicWaker::new());

        let ui_context = PluginUiContext::new(plugin.clone(), react_request_receiver, react_event_sender, event_waker.clone());
        let react_context = PluginReactContext::new(plugin.clone(), react_event_receiver, event_waker, react_request_sender);

        (react_context, ui_context)
    }

}

#[derive(Debug, Deserialize)]
struct Config {
    readonly_ui: Option<bool>,
    plugins: Option<Vec<PluginConfig>>,
}

#[derive(Debug, Deserialize)]
struct PluginConfig {
    id: String,
}

#[derive(Debug, Deserialize)]
struct PackageJson {
    plugin: PackageJsonPlugin,
}

#[derive(Debug, Deserialize)]
struct PackageJsonPlugin {
    entrypoints: Vec<PackageJsonPluginEntrypoint>,
}

#[derive(Debug, Deserialize)]
struct PackageJsonPluginEntrypoint {
    id: String,
    name: String,
    path: String,
}

pub struct PluginLoader;

impl PluginLoader {
    pub fn load_plugins(&self) -> Vec<Plugin> {
        let project_dirs = ProjectDirs::from("org", "placeholdername", "placeholdername").unwrap();

        let config_dir = project_dirs.config_dir();

        std::fs::create_dir_all(config_dir).unwrap();

        let config_file = config_dir.join("config.toml");
        let config_file_path = config_file.to_string_lossy().into_owned();
        let config_content = std::fs::read_to_string(config_file).context(config_file_path).unwrap();
        let config: Config = toml::from_str(&config_content).unwrap();

        let plugins: Vec<_> = config.plugins.unwrap()
            .into_iter()
            .map(|plugin| self.fetch_plugin(plugin))
            .collect();

        plugins
    }

    fn fetch_plugin(&self, plugin: PluginConfig) -> Plugin {
        // TODO fetch from git repo
        let plugin_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test_data/plugin");
        let js_path = plugin_dir.join("dist/view.js");
        let js_content = std::fs::read_to_string(js_path).unwrap();

        let package_path = plugin_dir.join("package.json");
        let package_content = std::fs::read_to_string(package_path).unwrap();
        let package_json: PackageJson = serde_json::from_str(&package_content).unwrap();

        let entrypoints: Vec<_> = package_json.plugin
            .entrypoints
            .into_iter()
            .map(|entrypoint| PluginEntrypoint::new(entrypoint.id, entrypoint.name, entrypoint.path))
            .collect();

        let mut js = HashMap::new();
        js.insert("view.js".into(), js_content);

        Plugin::new(&plugin.id, PluginCode::new(js, None), entrypoints)
    }

}

#[derive(Clone)]
pub struct Plugin {
    id: String,
    code: PluginCode,
    entrypoints: Vec<PluginEntrypoint>
}

impl Plugin {
    fn new(id: &str, code: PluginCode, entrypoints: Vec<PluginEntrypoint>) -> Self {
        Self {
            id: id.into(),
            code,
            entrypoints,
        }
    }

    pub fn id(&self) -> &String {
        &self.id
    }

    pub fn code(&self) -> &PluginCode {
        &self.code
    }
}

#[derive(Clone)]
pub struct PluginEntrypoint {
    id: String,
    name: String,
    path: String,
}

impl PluginEntrypoint {
    fn new(id: String, name: String, path: String) -> Self {
        Self {
            id,
            name,
            path,
        }
    }
}

#[derive(Clone)]
pub struct PluginCode {
    js: HashMap<String, String>,
    css: Option<String>,
}

impl PluginCode {
    fn new(js: HashMap<String, String>, css: Option<String>) -> Self {
        Self {
            js,
            css,
        }
    }

    pub fn js(&self) -> &HashMap<String, String> {
        &self.js
    }

    pub fn css(&self) -> &Option<String> {
        &self.css
    }
}