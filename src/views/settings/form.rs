use camunda_core::camunda::{connection_tester, http};
use camunda_core::connection;
use camunda_core::models::{ConnectionDraft, Environment};
use gpui_kit::component::alert::Alert;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::form::{field, v_form};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::component::{ActiveTheme, Sizable, WindowExt as _, h_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::SettingsView;
use crate::platform::environment;
use crate::platform::services::{self, Services};

pub struct ConnectionForm {
    name: Entity<InputState>,
    rest_url: Entity<InputState>,
    environment: Environment,
    error: Option<String>,
    tested: Option<Result<Option<String>, String>>,
    testing: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

pub struct Saved;

impl EventEmitter<Saved> for ConnectionForm {}

impl SettingsView {
    // Новое подключение в диалоге GPUI Kit.
    pub(super) fn open_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let form = cx.new(|cx| ConnectionForm::new(window, cx));
        self._form = Some(cx.subscribe_in(&form, window, |this, _, _: &Saved, window, cx| {
            window.close_dialog(cx);
            this.reload_connections(cx);
        }));
        window.open_dialog(cx, move |dialog, _, _| dialog.title("Новое подключение").w(px(560.)).child(form.clone()));
    }
}

impl ConnectionForm {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("Например: Залоги TEST"));
        let rest_url = cx.new(|cx| InputState::new(window, cx).placeholder("https://camunda.example.kz/engine-rest"));
        let reset = |this: &mut Self, _: Entity<InputState>, event: &InputEvent, cx: &mut Context<Self>| {
            if matches!(event, InputEvent::Change) {
                this.error = None;
                this.tested = None;
                cx.notify();
            }
        };
        let _subscriptions = vec![cx.subscribe(&name, reset), cx.subscribe(&rest_url, reset)];
        Self { name, rest_url, environment: Environment::Test, error: None, tested: None, testing: None, _subscriptions }
    }

    fn draft(&self, cx: &App) -> ConnectionDraft {
        let rest_url = self.rest_url.read(cx).value().trim().to_string();
        ConnectionDraft {
            name: self.name.read(cx).value().trim().to_string(),
            cockpit_url: connection::derive_cockpit_url(&rest_url),
            rest_url,
            environment: self.environment,
        }
    }

    fn test(&mut self, cx: &mut Context<Self>) {
        let rest_url = self.draft(cx).rest_url;
        let check = services::request(cx, move |_| async move {
            connection_tester::test_connection(&http::camunda_client(), &rest_url).await.map(|success| success.engine_version)
        });
        self.tested = None;
        self.testing = Some(cx.spawn(async move |this, cx| {
            let result = check.await;
            this.update(cx, |this, cx| {
                this.tested = Some(result);
                this.testing = None;
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    fn save(&mut self, cx: &mut Context<Self>) {
        let draft = self.draft(cx);
        let validation = connection::validate_connection_draft(&draft);
        if !validation.is_valid() {
            self.error = validation.name_error.or(validation.rest_url_error).or(validation.cockpit_url_error);
            cx.notify();
            return;
        }
        let engine_version = self.tested.clone().and_then(Result::ok).flatten();
        let storage = cx.global::<Services>().storage.clone();
        let id = connection::connection_id(&draft.name, &storage.load_connections());
        storage.save_connection(&connection::draft_to_connection(&draft, id, engine_version));
        cx.emit(Saved);
    }
}

impl Render for ConnectionForm {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let cockpit = connection::derive_cockpit_url(self.rest_url.read(cx).value().trim());
        let selected = environment::ALL.iter().position(|candidate| *candidate == self.environment).unwrap_or(0);
        v_form()
            .child(field().label("Название").required(true).child(Input::new(&self.name)))
            .child(field().label("Адрес Camunda REST").required(true).child(Input::new(&self.rest_url)))
            .child(
                field().label("Адрес Cockpit").child(
                    div()
                        .text_sm()
                        .font_family(crate::platform::theme::mono(cx))
                        .text_color(cx.theme().muted_foreground)
                        .child(if cockpit.is_empty() { "—".to_string() } else { cockpit }),
                ),
            )
            .child(
                field().label("Окружение").child(
                    TabBar::new("environment")
                        .segmented()
                        .small()
                        .selected_index(selected)
                        .children(environment::ALL.iter().map(|environment| Tab::new().label(environment::label(environment))))
                        .on_click(cx.listener(|this, index: &usize, _, cx| {
                            this.environment = environment::ALL[*index];
                            cx.notify();
                        })),
                ),
            )
            .when_some(self.tested.clone(), |form, tested| {
                form.child(field().child(match tested {
                    Ok(version) => {
                        Alert::success("tested", format!("Подключение работает · Camunda {}", version.unwrap_or_else(|| "?".into())))
                    }
                    Err(error) => Alert::error("tested", error),
                }))
            })
            .when_some(self.error.clone(), |form, error| form.child(field().child(Alert::error("form-error", error))))
            .footer(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("test")
                            .label("Проверить")
                            .outline()
                            .loading(self.testing.is_some())
                            .on_click(cx.listener(|this, _, _, cx| this.test(cx))),
                    )
                    .child(div().flex_1())
                    .child(Button::new("save").label("Сохранить").primary().on_click(cx.listener(|this, _, _, cx| this.save(cx)))),
            )
    }
}
