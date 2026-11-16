use std::future::Future;

use camunda_core::camunda::{connection_tester, http};
use camunda_core::connection;
use camunda_core::models::{CamundaConnection, ConnectionDraft, Environment};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::group_box::{GroupBox, GroupBoxVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::switch::Switch;
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme, Sizable, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::platform::services::{self, Services};
use crate::updates::{Status, Updater, VERSION};

const ENVIRONMENTS: [(Environment, &str); 4] = [
    (Environment::Production, "Прод"),
    (Environment::Test, "Тест"),
    (Environment::Development, "Разработка"),
    (Environment::Local, "Локально"),
];

pub enum SettingsEvent {
    ConnectionsChanged,
}

pub struct SettingsView {
    connections: Vec<CamundaConnection>,
    form: Option<ConnectionForm>,
    pending_delete: Option<String>,
    _updates: Subscription,
}

struct ConnectionForm {
    name: Entity<InputState>,
    rest_url: Entity<InputState>,
    environment: Environment,
    error: Option<String>,
    tested: Option<Result<Option<String>, String>>,
    testing: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<SettingsEvent> for SettingsView {}

// Любая работа с сетью идёт на tokio через тот же runtime, что и запросы к Camunda.
fn on_runtime<T, F>(cx: &App, work: F) -> impl Future<Output = Result<T, String>> + use<T, F>
where
    T: Send + 'static,
    F: Future<Output = Result<T, String>> + Send + 'static,
{
    services::request(cx, move |_| work)
}

impl SettingsView {
    pub fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            connections: cx.global::<Services>().storage.load_connections(),
            form: None,
            pending_delete: None,
            _updates: cx.observe(&Updater::global(cx), |_, _, cx| cx.notify()),
        }
    }

    fn reload_connections(&mut self, cx: &mut Context<Self>) {
        self.connections = cx.global::<Services>().storage.load_connections();
        cx.emit(SettingsEvent::ConnectionsChanged);
        cx.notify();
    }

    fn open_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("Например: Залоги TEST"));
        let rest_url = cx.new(|cx| InputState::new(window, cx).placeholder("https://camunda.example.kz/engine-rest"));
        let reset = |this: &mut Self, _: Entity<InputState>, event: &InputEvent, cx: &mut Context<Self>| {
            if matches!(event, InputEvent::Change) {
                if let Some(form) = &mut this.form {
                    form.error = None;
                    form.tested = None;
                }
                cx.notify();
            }
        };
        let subscriptions = vec![cx.subscribe(&name, reset), cx.subscribe(&rest_url, reset)];
        self.form = Some(ConnectionForm {
            name,
            rest_url,
            environment: Environment::Test,
            error: None,
            tested: None,
            testing: None,
            _subscriptions: subscriptions,
        });
        cx.notify();
    }

    fn draft(&self, cx: &App) -> Option<ConnectionDraft> {
        let form = self.form.as_ref()?;
        let rest_url = form.rest_url.read(cx).value().trim().to_string();
        Some(ConnectionDraft {
            name: form.name.read(cx).value().trim().to_string(),
            cockpit_url: connection::derive_cockpit_url(&rest_url),
            rest_url,
            environment: form.environment,
        })
    }

    fn test_connection(&mut self, cx: &mut Context<Self>) {
        let Some(draft) = self.draft(cx) else { return };
        let rest_url = draft.rest_url.clone();
        let check = on_runtime(cx, async move {
            let client = http::camunda_client();
            connection_tester::test_connection(&client, &rest_url).await
        });
        let Some(form) = &mut self.form else { return };
        form.tested = None;
        form.testing = Some(cx.spawn(async move |this, cx| {
            let result = check.await;
            this.update(cx, |this, cx| {
                if let Some(form) = &mut this.form {
                    form.tested = Some(result.map(|success| success.engine_version));
                    form.testing = None;
                }
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    fn save_connection(&mut self, cx: &mut Context<Self>) {
        let Some(draft) = self.draft(cx) else { return };
        let validation = connection::validate_connection_draft(&draft);
        if !validation.is_valid() {
            if let Some(form) = &mut self.form {
                form.error = validation.name_error.or(validation.rest_url_error).or(validation.cockpit_url_error);
            }
            cx.notify();
            return;
        }
        let engine_version = self.form.as_ref().and_then(|form| form.tested.clone()).and_then(Result::ok).flatten();
        let storage = cx.global::<Services>().storage.clone();
        let id = connection::connection_id(&draft.name, &storage.load_connections());
        storage.save_connection(&connection::draft_to_connection(&draft, id, engine_version));
        self.form = None;
        self.reload_connections(cx);
    }

    fn delete_connection(&mut self, id: String, cx: &mut Context<Self>) {
        cx.global::<Services>().storage.delete_connection(&id);
        self.pending_delete = None;
        self.reload_connections(cx);
    }

    fn render_connections(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let head = h_flex()
            .w_full()
            .gap_2()
            .child(div().font_weight(FontWeight::SEMIBOLD).child("Подключения"))
            .child(Tag::secondary().small().rounded_full().child((self.connections.len()).to_string()))
            .child(div().flex_1())
            .child(
                Button::new("add-connection")
                    .label("Добавить")
                    .small()
                    .primary()
                    .on_click(cx.listener(|this, _, window, cx| this.open_form(window, cx))),
            );
        GroupBox::new()
            .outline()
            .title(head)
            .content_style(crate::platform::theme::panel())
            .flex_none()
            .children(self.connections.iter().map(|connection| {
                let id = connection.id.clone();
                let confirming = self.pending_delete.as_deref() == Some(connection.id.as_str());
                h_flex()
                    .px_4()
                    .py_3()
                    .gap_3()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(div().size(px(8.)).flex_none().rounded_full().bg(environment_color(&connection.environment, cx)))
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap_0p5()
                            .child(h_flex().gap_2().child(div().font_weight(FontWeight::MEDIUM).child(connection.name.clone())).child(
                                div().text_sm().text_color(cx.theme().muted_foreground).child(format!(
                                    "{} · Camunda {}",
                                    environment_label(&connection.environment),
                                    connection.engine_version.clone().unwrap_or_else(|| "?".into())
                                )),
                            ))
                            .child(
                                div()
                                    .truncate()
                                    .text_xs()
                                    .font_family(crate::platform::theme::mono(cx))
                                    .text_color(cx.theme().muted_foreground)
                                    .child(connection.rest_url.clone()),
                            ),
                    )
                    .map(|row| {
                        if confirming {
                            let target = id.clone();
                            row.child(div().text_sm().text_color(cx.theme().danger).child("Удалить подключение?"))
                                .child(Button::new(SharedString::from(format!("cancel-{id}"))).label("Нет").ghost().small().on_click(
                                    cx.listener(|this, _, _, cx| {
                                        this.pending_delete = None;
                                        cx.notify();
                                    }),
                                ))
                                .child(
                                    Button::new(SharedString::from(format!("confirm-{id}")))
                                        .label("Удалить")
                                        .danger()
                                        .small()
                                        .on_click(cx.listener(move |this, _, _, cx| this.delete_connection(target.clone(), cx))),
                                )
                        } else {
                            row.child(Button::new(SharedString::from(format!("delete-{id}"))).label("Удалить").ghost().small().on_click(
                                cx.listener(move |this, _, _, cx| {
                                    this.pending_delete = Some(id.clone());
                                    cx.notify();
                                }),
                            ))
                        }
                    })
            }))
            .when(self.connections.is_empty(), |this| {
                this.child(div().px_4().py_6().text_color(cx.theme().muted_foreground).child("Подключений пока нет — добавьте первое."))
            })
    }

    fn render_appearance(&self, cx: &mut Context<Self>) -> impl IntoElement {
        GroupBox::new().outline().title("Вид").child(
            Switch::new("dark-theme")
                .label("Тёмная тема")
                .checked(cx.theme().is_dark())
                .on_click(|_, window, cx| crate::platform::theme::toggle(window, cx)),
        )
    }

    fn render_updates(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let updater = Updater::global(cx);
        let updater = updater.read(cx);
        let status = updater.status();
        let action = match status {
            Status::Ready { .. } => {
                Some(Button::new("restart").label("Перезапустить").primary().small().on_click(|_, _, cx| Updater::restart(cx)))
            }
            status if status.is_busy() => None,
            _ => Some(Button::new("check-update").label("Проверить").small().on_click(|_, _, cx| {
                Updater::global(cx).update(cx, |updater, cx| updater.check(cx));
            })),
        };
        let description = status.describe(updater.progress().as_ref());
        let failed = matches!(status, Status::Failed(_));
        GroupBox::new().outline().title("Обновления").child(
            h_flex()
                .gap_3()
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap_0p5()
                        .child(div().font_weight(FontWeight::MEDIUM).child(format!("Версия {VERSION}")))
                        .child(
                            div()
                                .text_sm()
                                .text_color(if failed { cx.theme().danger } else { cx.theme().muted_foreground })
                                .child(description),
                        ),
                )
                .children(action),
        )
    }

    fn render_form(&self, form: &ConnectionForm, cx: &mut Context<Self>) -> impl IntoElement {
        let derived = connection::derive_cockpit_url(form.rest_url.read(cx).value().trim());
        let testing = form.testing.is_some();
        let field = |label: &'static str, input: AnyElement| {
            v_flex().gap_1p5().child(div().text_sm().font_weight(FontWeight::MEDIUM).child(label)).child(input)
        };
        v_flex()
            .id("connection-form")
            .w(px(560.))
            .p_5()
            .gap_4()
            .rounded_lg()
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().popover)
            .shadow_2xl()
            .occlude()
            .child(v_flex().gap_1().child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child("Новое подключение")).child(
                div().text_sm().text_color(cx.theme().muted_foreground).child("Адрес Cockpit построится из адреса REST автоматически."),
            ))
            .child(field("Название подключения", Input::new(&form.name).into_any_element()))
            .child(field("Адрес Camunda REST", Input::new(&form.rest_url).into_any_element()))
            .child(field(
                "Адрес Cockpit",
                div()
                    .text_sm()
                    .font_family(crate::platform::theme::mono(cx))
                    .text_color(cx.theme().muted_foreground)
                    .child(if derived.is_empty() { "—".to_string() } else { derived })
                    .into_any_element(),
            ))
            .child(field(
                "Окружение",
                h_flex()
                    .gap_1()
                    .children(ENVIRONMENTS.into_iter().map(|(environment, label)| {
                        Button::new(label)
                            .label(label)
                            .small()
                            .map(|button| if form.environment == environment { button.secondary() } else { button.ghost() })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if let Some(form) = &mut this.form {
                                    form.environment = environment;
                                }
                                cx.notify();
                            }))
                    }))
                    .into_any_element(),
            ))
            .when_some(form.tested.clone(), |this, tested| {
                let (text, color) = match tested {
                    Ok(version) => {
                        (format!("Подключение работает · Camunda {}", version.unwrap_or_else(|| "?".into())), cx.theme().success)
                    }
                    Err(error) => (error, cx.theme().danger),
                };
                this.child(div().px_3().py_2().rounded_md().bg(color.opacity(0.1)).text_sm().text_color(color).child(text))
            })
            .when_some(form.error.clone(), |this, error| {
                this.child(
                    div()
                        .px_3()
                        .py_2()
                        .rounded_md()
                        .bg(cx.theme().danger.opacity(0.1))
                        .text_sm()
                        .text_color(cx.theme().danger)
                        .child(error),
                )
            })
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("test-connection")
                            .label("Проверить")
                            .small()
                            .loading(testing)
                            .on_click(cx.listener(|this, _, _, cx| this.test_connection(cx))),
                    )
                    .child(div().flex_1())
                    .child(Button::new("cancel-form").label("Отмена").ghost().on_click(cx.listener(|this, _, _, cx| {
                        this.form = None;
                        cx.notify();
                    })))
                    .child(
                        Button::new("save-form")
                            .label("Сохранить")
                            .primary()
                            .on_click(cx.listener(|this, _, _, cx| this.save_connection(cx))),
                    ),
            )
    }
}

fn environment_label(environment: &Environment) -> &'static str {
    ENVIRONMENTS.iter().find(|(candidate, _)| candidate == environment).map(|(_, label)| *label).unwrap_or("—")
}

fn environment_color(environment: &Environment, cx: &App) -> Hsla {
    match environment {
        Environment::Production => cx.theme().danger,
        Environment::Test => cx.theme().warning,
        Environment::Development => cx.theme().info,
        Environment::Local => cx.theme().muted_foreground,
    }
}

impl Render for SettingsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let form = self.form.as_ref().map(|form| self.render_form(form, cx).into_any_element());
        div()
            .relative()
            .size_full()
            .child(
                v_flex().id("settings").size_full().overflow_y_scroll().p_5().child(
                    v_flex()
                        .w_full()
                        .max_w(px(820.))
                        .gap_5()
                        .child(div().text_xl().font_weight(FontWeight::SEMIBOLD).child("Настройки"))
                        .child(self.render_connections(cx))
                        .child(self.render_appearance(cx))
                        .child(self.render_updates(cx)),
                ),
            )
            .when_some(form, |this, form| {
                this.child(deferred(
                    div().absolute().inset_0().flex().items_center().justify_center().p_6().bg(black().opacity(0.45)).child(form),
                ))
            })
    }
}
