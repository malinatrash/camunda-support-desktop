mod palette;

use camunda_core::deep_link;
use camunda_core::models::{CamundaConnection, DeepLinkEvent, Environment};
use futures::StreamExt as _;
use futures::channel::mpsc::UnboundedReceiver;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::kbd::Kbd;
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::{ActiveTheme, IconName, Sizable, TitleBar, WindowExt as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::model::Navigate;
use crate::platform::services::Services;
use crate::views::definition::DefinitionView;
use crate::views::incidents::IncidentsView;
use crate::views::instance::InstanceView;
use crate::views::overview::OverviewView;
use crate::views::search::SearchView;
use crate::views::settings::{SettingsEvent, SettingsView};
use palette::{CommandPalette, PaletteEvent, Section};

actions!(camunda_support, [TogglePalette, OpenOverview, OpenSearch, OpenIncidents, OpenSettings,]);

enum Page {
    Definition(Entity<DefinitionView>),
    Instance(Entity<InstanceView>),
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
        let connections = cx.global::<Services>().storage.load_connections();
        let focus = cx.focus_handle();
        focus.focus(window, cx);
        let links = cx.spawn_in(window, async move |this, cx| {
            let mut links = links;
            while let Some(url) = links.next().await {
                if this.update_in(cx, |this, window, cx| this.open_link(&url, window, cx)).is_err() {
                    break;
                }
            }
        });
        let updates = crate::updates::announce(window, cx);
        let mut view = Self {
            connections,
            connection: None,
            section: Section::Overview,
            overview: None,
            search: None,
            incidents: None,
            settings: None,
            pages: Vec::new(),
            focus,
            _section_subscriptions: Vec::new(),
            _page_subscriptions: Vec::new(),
            _palette_subscription: None,
            _settings_subscription: None,
            _links: links,
            _updates: updates,
        };
        let preferred = std::env::var("CAMUNDA_SUPPORT_CONNECTION").ok();
        let initial = view
            .connections
            .iter()
            .find(|connection| preferred.as_deref() == Some(connection.name.as_str()))
            .or(view.connections.first())
            .cloned();
        match initial {
            Some(connection) => view.select_connection(connection, window, cx),
            None => view.section = Section::Settings,
        }
        if let Ok(id) = std::env::var("CAMUNDA_SUPPORT_OPEN_INSTANCE") {
            view.open_page(Navigate::Instance(id), window, cx);
        }
        view
    }

    fn select_connection(&mut self, connection: CamundaConnection, window: &mut Window, cx: &mut Context<Self>) {
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

    fn navigate<V>(&mut self, _: &Entity<V>, event: &Navigate, window: &mut Window, cx: &mut Context<Self>) {
        self.open_page(event.clone(), window, cx);
    }

    fn open_page(&mut self, event: Navigate, window: &mut Window, cx: &mut Context<Self>) {
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

    fn go(&mut self, section: Section, window: &mut Window, cx: &mut Context<Self>) {
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

    fn toggle_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) {
            window.close_dialog(cx);
            return;
        }
        let palette = cx.new(|cx| CommandPalette::new(self.connection.clone(), window, cx));
        self._palette_subscription = Some(cx.subscribe_in(&palette, window, |this, _, event: &PaletteEvent, window, cx| {
            window.close_dialog(cx);
            match event {
                PaletteEvent::Open(id) => {
                    this.pages.clear();
                    this.open_page(Navigate::Instance(id.clone()), window, cx);
                }
                PaletteEvent::Go(section) => this.go(*section, window, cx),
            }
            this.focus.focus(window, cx);
        }));
        window.open_dialog(cx, move |dialog, _, _| dialog.w(px(640.)).p_0().close_button(false).child(palette.clone()));
    }

    // Ссылка camunda-support:// открывает заявку в подключении с тем же доменом.
    fn open_link(&mut self, url: &str, window: &mut Window, cx: &mut Context<Self>) {
        window.activate_window();
        match deep_link::parse_deep_link(url) {
            DeepLinkEvent::OpenProcessInstance { link } => {
                let Some(connection) = self.connections.iter().find(|c| deep_link::matches_origin(c, &link.origin)).cloned() else {
                    window.push_notification(
                        Notification::warning(format!("Нет подключения для {}. Добавьте его в настройках.", link.origin)),
                        cx,
                    );
                    return;
                };
                if self.connection.as_ref().map(|c| &c.id) != Some(&connection.id) {
                    self.select_connection(connection, window, cx);
                }
                self.section = Section::Overview;
                self.pages.clear();
                self.open_page(Navigate::Instance(link.process_instance_id), window, cx);
            }
            DeepLinkEvent::Invalid { message } => {
                window.push_notification(Notification::error(message), cx);
            }
        }
    }

    fn render_switcher(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let name = self.connection.as_ref().map(|connection| connection.name.clone()).unwrap_or_else(|| "Нет подключения".into());
        let tone = self
            .connection
            .as_ref()
            .map(|connection| environment_color(&connection.environment, cx))
            .unwrap_or(cx.theme().muted_foreground);
        let connections = self.connections.clone();
        let current = self.connection.as_ref().map(|connection| connection.id.clone());
        let view = cx.entity().downgrade();
        Button::new("connection")
            .ghost()
            .small()
            .dropdown_caret(true)
            .child(h_flex().gap_2().child(div().size(px(7.)).rounded_full().bg(tone)).child(name))
            .dropdown_menu(move |menu, _, _| {
                let menu = connections.iter().fold(menu.min_w(px(320.)), |menu, connection| {
                    let target = connection.clone();
                    let view = view.clone();
                    menu.item(
                        PopupMenuItem::new(format!("{} · {}", connection.name, connection.rest_url))
                            .checked(current.as_deref() == Some(connection.id.as_str()))
                            .on_click(move |_, window, cx| {
                                view.update(cx, |this, cx| this.select_connection(target.clone(), window, cx)).ok();
                            }),
                    )
                });
                let view = view.clone();
                menu.separator().item(PopupMenuItem::new("Управлять подключениями…").on_click(move |_, window, cx| {
                    view.update(cx, |this, cx| this.go(Section::Settings, window, cx)).ok();
                }))
            })
    }

    fn nav_button(&self, section: Section, label: &'static str, cx: &mut Context<Self>) -> impl IntoElement {
        let active = self.section == section && self.pages.is_empty();
        Button::new(label)
            .label(label)
            .small()
            .map(|button| if active { button.secondary() } else { button.ghost() })
            .on_click(cx.listener(move |this, _, window, cx| this.go(section, window, cx)))
    }
}

fn environment_color(environment: &Environment, cx: &App) -> Hsla {
    match environment {
        Environment::Production => cx.theme().danger,
        Environment::Test => cx.theme().warning,
        Environment::Development => cx.theme().info,
        Environment::Local => cx.theme().muted_foreground,
    }
}

impl Render for AppView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let section_view = match self.section {
            Section::Overview => self.overview.clone().map(|view| view.into_any_element()),
            Section::Search => self.search.clone().map(|view| view.into_any_element()),
            Section::Incidents => self.incidents.clone().map(|view| view.into_any_element()),
            Section::Settings => self.settings.clone().map(|view| view.into_any_element()),
        };
        let content = match self.pages.last() {
            Some(Page::Definition(page)) => page.clone().into_any_element(),
            Some(Page::Instance(page)) => page.clone().into_any_element(),
            None => section_view.unwrap_or_else(|| {
                div()
                    .p_6()
                    .text_color(cx.theme().muted_foreground)
                    .child("Подключений нет. Добавьте первое в настройках.")
                    .into_any_element()
            }),
        };
        v_flex()
            .id("app")
            .key_context("CamundaSupport")
            .track_focus(&self.focus)
            .relative()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .on_action(cx.listener(|this, _: &TogglePalette, window, cx| this.toggle_palette(window, cx)))
            .on_action(cx.listener(|this, _: &OpenOverview, window, cx| this.go(Section::Overview, window, cx)))
            .on_action(cx.listener(|this, _: &OpenSearch, window, cx| this.go(Section::Search, window, cx)))
            .on_action(cx.listener(|this, _: &OpenIncidents, window, cx| this.go(Section::Incidents, window, cx)))
            .on_action(cx.listener(|this, _: &OpenSettings, window, cx| this.go(Section::Settings, window, cx)))
            .child(
                TitleBar::new().child(
                    h_flex()
                        .w_full()
                        .pr_3()
                        .gap_2()
                        .child(self.render_switcher(cx))
                        .child(div().w(px(8.)))
                        .child(self.nav_button(Section::Overview, "Обзор", cx))
                        .child(self.nav_button(Section::Search, "Поиск", cx))
                        .child(self.nav_button(Section::Incidents, "Инциденты", cx))
                        .child(div().flex_1())
                        .child(
                            Button::new("palette-trigger")
                                .outline()
                                .small()
                                .w(px(260.))
                                .icon(IconName::Search)
                                .child(
                                    h_flex()
                                        .flex_1()
                                        .justify_between()
                                        .child("Найти заявку")
                                        .child(Kbd::new(Keystroke::parse("cmd-k").expect("сочетание"))),
                                )
                                .on_click(cx.listener(|this, _, window, cx| this.toggle_palette(window, cx))),
                        )
                        .child(
                            Button::new("theme-toggle")
                                .ghost()
                                .small()
                                .icon(if cx.theme().is_dark() { gpui_kit::assets::IconName::Sun } else { gpui_kit::assets::IconName::Moon })
                                .tooltip(if cx.theme().is_dark() { "Светлая тема" } else { "Тёмная тема" })
                                .on_click(|_, window, cx| crate::platform::theme::toggle(window, cx)),
                        )
                        .child(self.nav_button(Section::Settings, "Настройки", cx)),
                ),
            )
            .child(div().flex_1().min_h_0().child(content))
    }
}
