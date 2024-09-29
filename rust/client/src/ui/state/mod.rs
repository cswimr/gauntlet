mod main_view;

use crate::ui::scroll_handle::ScrollHandle;
pub use crate::ui::state::main_view::MainViewState;
use crate::ui::AppMsg;
use common::model::{EntrypointId, PhysicalShortcut, PluginId, SearchResult};
use iced::widget::text_input;
use iced::widget::text_input::focus;
use iced::Command;
use std::collections::HashMap;

pub enum GlobalState {
    MainView {
        // logic
        search_field_id: text_input::Id,

        // ephemeral state
        prompt: String,
        focused_search_result: ScrollHandle<SearchResult>,
        sub_state: MainViewState,

        // state
        pending_plugin_view_data: Option<PluginViewData>,
        search_results: Vec<SearchResult>,
    },
    ErrorView {
        error_view: ErrorViewData,
    },
    PluginView(PluginViewData)
}

#[derive(Clone)]
pub struct PluginViewData {
    pub top_level_view: bool,
    pub plugin_id: PluginId,
    pub plugin_name: String,
    pub entrypoint_id: EntrypointId,
    pub entrypoint_name: String,
    pub action_shortcuts: HashMap<String, PhysicalShortcut>,
}

pub enum ErrorViewData {
    PreferenceRequired {
        plugin_id: PluginId,
        entrypoint_id: EntrypointId,
        plugin_preferences_required: bool,
        entrypoint_preferences_required: bool,
    },
    PluginError {
        plugin_id: PluginId,
        entrypoint_id: EntrypointId,
    },
    BackendTimeout,
    UnknownError {
        display: String
    }
}

impl GlobalState {
    pub fn new(search_field_id: text_input::Id) -> GlobalState {
        GlobalState::MainView {
            search_field_id,
            prompt: "".to_string(),
            focused_search_result: ScrollHandle::new(),
            sub_state: MainViewState::new(),
            pending_plugin_view_data: None,
            search_results: vec![]
        }
    }

    pub fn new_error(error_view_data: ErrorViewData) -> GlobalState {
        GlobalState::ErrorView {
            error_view: error_view_data,
        }
    }

    pub fn new_plugin(plugin_view_data: PluginViewData) -> GlobalState {
        GlobalState::PluginView(plugin_view_data)
    }

    pub fn initial(prev_global_state: &mut GlobalState) -> Command<AppMsg> {
        let search_field_id = text_input::Id::unique();

        *prev_global_state = GlobalState::new(search_field_id.clone());

        Command::batch([
            focus(search_field_id),
            Command::perform(async {}, |_| AppMsg::PromptChanged("".to_owned())),
        ])
    }

    pub fn error(prev_global_state: &mut GlobalState, error_view_data: ErrorViewData) -> Command<AppMsg> {
        *prev_global_state = GlobalState::ErrorView {
            error_view: error_view_data,
        };

        Command::none()
    }

    pub fn plugin(prev_global_state: &mut GlobalState, plugin_view_data: PluginViewData) -> Command<AppMsg> {
        *prev_global_state = GlobalState::PluginView(plugin_view_data);

        Command::none()
    }
}

pub trait Focus {
    fn enter(&mut self) -> Command<AppMsg>;
    fn escape(&mut self) -> Command<AppMsg>;
    fn tab(&mut self) -> Command<AppMsg>;
    fn shift_tab(&mut self) -> Command<AppMsg>;
    fn arrow_up(&mut self) -> Command<AppMsg>;
    fn arrow_down(&mut self) -> Command<AppMsg>;
    fn arrow_left(&mut self) -> Command<AppMsg>;
    fn arrow_right(&mut self) -> Command<AppMsg>;
}

impl Focus for GlobalState {
    fn enter(&mut self) -> Command<AppMsg> {
        match self {
            GlobalState::MainView { focused_search_result, sub_state, search_results, .. } => {
                match sub_state {
                    MainViewState::None => {
                        if let Some(search_item) = focused_search_result.get(search_results) {
                            let search_item = search_item.clone();
                            Command::perform(async {}, |_| AppMsg::RunSearchItemAction(search_item, None))
                        } else {
                            Command::none()
                        }
                    }
                    MainViewState::ActionPanel { focused_action_item, .. } => {
                        let widget_id = focused_action_item.index;

                        MainViewState::initial(sub_state);

                        Command::perform(async {}, move |_| AppMsg::OnEntrypointAction(widget_id))
                    }
                }
            }
            GlobalState::PluginView(_) => {
                todo!()
            }
            GlobalState::ErrorView { .. } => Command::none()
        }
    }

    fn escape(&mut self) -> Command<AppMsg> {
        match self {
            GlobalState::MainView { sub_state, .. } => {
                match sub_state {
                    MainViewState::None => {
                        Command::perform(async {}, |_| AppMsg::HideWindow)
                    }
                    MainViewState::ActionPanel { .. } => {
                        MainViewState::initial(sub_state);
                        Command::none()
                    }
                }
            }
            GlobalState::PluginView(PluginViewData { top_level_view: true, plugin_id, .. }) => {
                let plugin_id = plugin_id.clone();

                Command::batch([
                    Command::perform(async {}, |_| AppMsg::ClosePluginView(plugin_id)),
                    GlobalState::initial(self)
                ])
            }
            GlobalState::PluginView(PluginViewData { top_level_view: false, plugin_id, entrypoint_id, .. }) => {
                let plugin_id= plugin_id.clone();
                let entrypoint_id = entrypoint_id.clone();
                Command::perform(async {}, |_| AppMsg::OpenPluginView(plugin_id, entrypoint_id))
            }
            GlobalState::ErrorView { .. } => {
                Command::perform(async {}, |_| AppMsg::HideWindow)
            }
        }
    }
    fn tab(&mut self) -> Command<AppMsg> {
        match self {
            GlobalState::MainView { .. } => Command::none(),
            GlobalState::PluginView(_) => Command::none(),
            GlobalState::ErrorView { .. } => Command::none(),
        }
    }
    fn shift_tab(&mut self) -> Command<AppMsg> {
        match self {
            GlobalState::MainView { .. } => Command::none(),
            GlobalState::PluginView(_) => Command::none(),
            GlobalState::ErrorView { .. } => Command::none(),
        }
    }
    fn arrow_up(&mut self) -> Command<AppMsg> {
        match self {
            GlobalState::MainView { focused_search_result, sub_state, .. } => {
                if sub_state.is_none() {
                    focused_search_result.focus_previous()
                } else {
                    sub_state.arrow_up()
                }
            }
            GlobalState::ErrorView { .. } => Command::none(),
            GlobalState::PluginView(_) => Command::none(),
        }
    }
    fn arrow_down(&mut self) -> Command<AppMsg> {
        match self {
            GlobalState::MainView { focused_search_result, search_results, sub_state, .. } => {
                if sub_state.is_none() {
                    if search_results.len() != 0 {
                        focused_search_result.focus_next(search_results.len())
                    } else {
                        Command::none()
                    }
                } else {
                    sub_state.arrow_down()
                }
            }
            GlobalState::ErrorView { .. } => Command::none(),
            GlobalState::PluginView(_) => Command::none(),
        }
    }
    fn arrow_left(&mut self) -> Command<AppMsg> {
        match self {
            GlobalState::MainView { .. } => Command::none(),
            GlobalState::PluginView(_) => Command::none(),
            GlobalState::ErrorView { .. } => Command::none(),
        }
    }
    fn arrow_right(&mut self) -> Command<AppMsg> {
        match self {
            GlobalState::MainView { .. } => Command::none(),
            GlobalState::PluginView(_) => Command::none(),
            GlobalState::ErrorView { .. } => Command::none(),
        }
    }
}
