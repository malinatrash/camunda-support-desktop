//! Настройки на компоненте Settings из GPUI Kit: подключения, вид, обновления.

mod appearance;
mod connections;
mod form;
mod updates;

use camunda_core::models::CamundaConnection;
use gpui_kit::component::setting::Settings;
use gpui_kit::*;

use crate::platform::services::Services;
use crate::updates::Updater;

pub enum SettingsEvent {
    ConnectionsChanged,
}

pub struct SettingsView {
    connections: Vec<CamundaConnection>,
    _form: Option<Subscription>,
    _updates: Subscription,
}

impl EventEmitter<SettingsEvent> for SettingsView {}

impl SettingsView {
    pub fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            connections: cx.global::<Services>().storage.load_connections(),
            _form: None,
            _updates: cx.observe(&Updater::global(cx), |_, _, cx| cx.notify()),
        }
    }

    fn reload_connections(&mut self, cx: &mut Context<Self>) {
        self.connections = cx.global::<Services>().storage.load_connections();
        cx.emit(SettingsEvent::ConnectionsChanged);
        cx.notify();
    }
}

impl Render for SettingsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Settings::new("settings").pages([self.connections_page(cx), appearance::page(), updates::page()])
    }
}
