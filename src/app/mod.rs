//! Оболочка окна: верхняя панель, разделы, стек страниц и палитра ⌘K.

mod actions;
mod connection_menu;
mod deep_link;
mod navigation;
mod palette;
mod sidebar;
mod status;
mod toolbar;

use camunda_core::models::CamundaConnection;
use futures::channel::mpsc::UnboundedReceiver;
use gpui_kit::component::{ActiveTheme, h_flex, v_flex};
use gpui_kit::*;

use crate::platform::services::Services;
use crate::platform::{motion, theme};
use crate::views::definition::DefinitionView;
use crate::views::incidents::IncidentsView;
use crate::views::instance::InstanceView;
use crate::views::overview::OverviewView;
use crate::views::search::SearchView;
use crate::views::settings::SettingsView;
pub use actions::bind_keys;
use actions::{OpenIncidents, OpenOverview, OpenSearch, OpenSettings, TogglePalette, ToggleSidebar};
use palette::Section;

enum Page {
    Definition(Entity<DefinitionView>, SharedString),
    Instance(Entity<InstanceView>, SharedString),
}

impl Page {
    fn label(&self) -> SharedString {
        match self {
            Page::Definition(_, label) | Page::Instance(_, label) => label.clone(),
        }
    }
}

pub struct AppView {
    connections: Vec<CamundaConnection>,
    connection: Option<CamundaConnection>,
    section: Section,
    overview: Option<Entity<OverviewView>>,
    search: Option<Entity<SearchView>>,
    incidents: Option<Entity<IncidentsView>>,
    settings: Option<Entity<SettingsView>>,
    pages: Vec<Page>,
    route: usize,
    sidebar_collapsed: bool,
    popover_generation: usize,
    focus: FocusHandle,
    _section_subscriptions: Vec<Subscription>,
    _page_subscriptions: Vec<Subscription>,
    _palette_subscription: Option<Subscription>,
    _settings_subscription: Option<Subscription>,
    _links: Task<()>,
    _updates: Subscription,
}

impl AppView {
    pub fn new(links: UnboundedReceiver<String>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        focus.focus(window, cx);
        let mut view = Self {
            connections: cx.global::<Services>().storage.load_connections(),
            connection: None,
            section: Section::Overview,
            overview: None,
            search: None,
            incidents: None,
            settings: None,
            pages: Vec::new(),
            route: 0,
            sidebar_collapsed: false,
            popover_generation: 0,
            focus,
            _section_subscriptions: Vec::new(),
            _page_subscriptions: Vec::new(),
            _palette_subscription: None,
            _settings_subscription: None,
            _links: deep_link::listen(links, window, cx),
            _updates: crate::updates::announce(window, cx),
        };
        view.open_initial(window, cx);
        view
    }

    fn content(&self, cx: &mut Context<Self>) -> AnyElement {
        let section = match self.section {
            Section::Overview => self.overview.clone().map(|view| view.into_any_element()),
            Section::Search => self.search.clone().map(|view| view.into_any_element()),
            Section::Incidents => self.incidents.clone().map(|view| view.into_any_element()),
            Section::Settings => self.settings.clone().map(|view| view.into_any_element()),
        };
        match self.pages.last() {
            Some(Page::Definition(page, _)) => page.clone().into_any_element(),
            Some(Page::Instance(page, _)) => page.clone().into_any_element(),
            None => section.unwrap_or_else(|| self.render_welcome(cx).into_any_element()),
        }
    }
}

impl Render for AppView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .id("app")
            .key_context("CamundaSupport")
            .track_focus(&self.focus)
            .size_full()
            .text_color(cx.theme().foreground)
            .on_action(cx.listener(|this, _: &TogglePalette, window, cx| this.toggle_palette(window, cx)))
            .on_action(cx.listener(|this, _: &ToggleSidebar, _, cx| this.toggle_sidebar(cx)))
            .on_action(cx.listener(|this, _: &OpenOverview, window, cx| this.go(Section::Overview, window, cx)))
            .on_action(cx.listener(|this, _: &OpenSearch, window, cx| this.go(Section::Search, window, cx)))
            .on_action(cx.listener(|this, _: &OpenIncidents, window, cx| this.go(Section::Incidents, window, cx)))
            .on_action(cx.listener(|this, _: &OpenSettings, window, cx| this.go(Section::Settings, window, cx)))
            .child(self.render_sidebar(cx))
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .bg(theme::surface(cx))
                    .child(self.render_toolbar(cx))
                    .child(div().flex_1().min_h_0().overflow_hidden().child(motion::page(self.route, div().relative().size_full().child(self.content(cx)))))
                    .child(self.render_status(cx)),
            )
    }
}
