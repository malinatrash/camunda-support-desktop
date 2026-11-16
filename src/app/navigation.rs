use camunda_core::models::CamundaConnection;
use gpui_kit::*;

use super::palette::Section;
use super::{AppView, Page};
use crate::model::Navigate;
use crate::platform::services::Services;
use crate::views::definition::DefinitionView;
use crate::views::incidents::IncidentsView;
use crate::views::instance::InstanceView;
use crate::views::overview::OverviewView;
use crate::views::search::SearchView;
use crate::views::settings::{SettingsEvent, SettingsView};

impl AppView {
    pub(super) fn go(&mut self, section: Section, window: &mut Window, cx: &mut Context<Self>) {
        self.section = section;
        self.pages.clear();
        self._page_subscriptions.clear();
        if section == Section::Settings && self.settings.is_none() {
            let settings = cx.new(|cx| SettingsView::new(window, cx));
            self._settings_subscription = Some(cx.subscribe_in(&settings, window, |this, _, event: &SettingsEvent, window, cx| {
                if matches!(event, SettingsEvent::ConnectionsChanged) {
                    this.reload_connections(window, cx);
                }
            }));
            self.settings = Some(settings);
        }
        cx.notify();
    }

    pub(super) fn open_page(&mut self, event: Navigate, window: &mut Window, cx: &mut Context<Self>) {
        let Some(connection) = self.connection.clone() else { return };
        match event {
            Navigate::Back => {
                self.pages.pop();
            }
            Navigate::Definition { id, all_versions } => {
                let page = cx.new(|cx| DefinitionView::new(connection, id, all_versions, window, cx));
                self._page_subscriptions.push(cx.subscribe_in(&page, window, Self::navigate));
                self.pages.push(Page::Definition(page));
            }
            Navigate::Instance(id) => {
                let page = cx.new(|cx| InstanceView::new(connection, id, window, cx));
                self._page_subscriptions.push(cx.subscribe_in(&page, window, Self::navigate));
                self.pages.push(Page::Instance(page));
            }
        }
        cx.notify();
    }

    pub(super) fn select_connection(&mut self, connection: CamundaConnection, window: &mut Window, cx: &mut Context<Self>) {
        let overview = cx.new(|cx| OverviewView::new(connection.clone(), window, cx));
        let search = cx.new(|cx| SearchView::new(connection.clone(), window, cx));
        let incidents = cx.new(|cx| IncidentsView::new(connection.clone(), window, cx));
        self._section_subscriptions = vec![
            cx.subscribe_in(&overview, window, Self::navigate),
            cx.subscribe_in(&search, window, Self::navigate),
            cx.subscribe_in(&incidents, window, Self::navigate),
        ];
        self.overview = Some(overview);
        self.search = Some(search);
        self.incidents = Some(incidents);
        self.connection = Some(connection);
        if self.section == Section::Settings {
            self.section = Section::Overview;
        }
        self.pages.clear();
        self._page_subscriptions.clear();
        cx.notify();
    }

    // Подключение из CAMUNDA_SUPPORT_CONNECTION или первое; без подключений сразу настройки.
    pub(super) fn open_initial(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let preferred = std::env::var("CAMUNDA_SUPPORT_CONNECTION").ok();
        let initial = self
            .connections
            .iter()
            .find(|connection| preferred.as_deref() == Some(connection.name.as_str()))
            .or(self.connections.first())
            .cloned();
        match initial {
            Some(connection) => self.select_connection(connection, window, cx),
            None => self.section = Section::Settings,
        }
        if let Ok(id) = std::env::var("CAMUNDA_SUPPORT_OPEN_INSTANCE") {
            self.open_page(Navigate::Instance(id), window, cx);
        }
    }

    fn navigate<V>(&mut self, _: &Entity<V>, event: &Navigate, window: &mut Window, cx: &mut Context<Self>) {
        self.open_page(event.clone(), window, cx);
    }

    fn reload_connections(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.connections = cx.global::<Services>().storage.load_connections();
        let current = self.connection.as_ref().and_then(|current| self.connections.iter().find(|c| c.id == current.id)).cloned();
        match current.or_else(|| self.connections.first().cloned()) {
            Some(connection) if self.connection.as_ref().map(|c| &c.id) != Some(&connection.id) => {
                let section = self.section;
                self.select_connection(connection, window, cx);
                self.section = section;
            }
            Some(_) => {}
            None => {
                self.connection = None;
                self.overview = None;
                self.search = None;
                self.incidents = None;
            }
        }
        cx.notify();
    }
}
