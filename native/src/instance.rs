use std::time::Duration;

use camunda_core::deep_link;
use camunda_core::models::{CamundaConnection, Environment, ProcessInstanceDetails};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::tab::TabBar;
use gpui_kit::component::{ActiveTheme, Disableable, IconName, Sizable, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::bpmn::{BpmnView, Highlights, SelectNode};
use crate::model;
use crate::operations::{self, Operation};
use crate::services;
use crate::teleport::{TeleportEvent, TeleportView};
use crate::ui::{self, Navigate, Tone};
use crate::variables::{VariablesChanged, VariablesView};

const TABS: [&str; 5] = ["Схема", "Хронология", "Диагностика", "Переменные", "Метаданные"];
const REFRESH_EVERY: Duration = Duration::from_secs(30);

pub struct InstanceView {
    connection: CamundaConnection,
    instance_id: String,
    details: Option<ProcessInstanceDetails>,
    diagram: Option<Entity<BpmnView>>,
    variables: Option<Entity<VariablesView>>,
    teleport: Option<Entity<TeleportView>>,
    pending: Option<Operation>,
    operation: Option<Task<()>>,
    selected_node: Option<String>,
    tab: usize,
    notice: Option<SharedString>,
    error: Option<String>,
    load: Option<Task<()>>,
    _diagram_subscription: Option<Subscription>,
    _variables_subscription: Option<Subscription>,
    _teleport_subscription: Option<Subscription>,
    _poll: Option<Task<()>>,
}

impl EventEmitter<Navigate> for InstanceView {}

impl InstanceView {
    pub fn new(connection: CamundaConnection, instance_id: String, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut view = Self {
            connection,
            instance_id,
            details: None,
            diagram: None,
            variables: None,
            teleport: None,
            pending: None,
            operation: None,
            selected_node: None,
            tab: 0,
            notice: None,
            error: None,
            load: None,
            _diagram_subscription: None,
            _variables_subscription: None,
            _teleport_subscription: None,
            _poll: None,
        };
        view.reload(window, cx);
        view._poll = Some(cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor().timer(REFRESH_EVERY).await;
                let alive = this.update_in(cx, |this, window, cx| {
                    if this.load.is_none() && this.teleport.is_none() && this.pending.is_none() {
                        this.reload(window, cx);
                    }
                });
                if alive.is_err() {
                    break;
                }
            }
        }));
        view
    }

    fn reload(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let connection = self.connection.clone();
        let id = self.instance_id.clone();
        let request = services::request(cx, move |api| async move { api.load_process_instance_details(&connection, &id).await });
        self.load = Some(cx.spawn_in(window, async move |this, cx| {
            let result = request.await;
            this.update_in(cx, |this, window, cx| {
                match result {
                    Ok(details) => this.show(details, window, cx),
                    Err(error) => this.error = Some(error),
                }
                this.load = None;
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    fn show(&mut self, details: ProcessInstanceDetails, window: &mut Window, cx: &mut Context<Self>) {
        let mut highlights = Highlights::default();
        highlights.active.extend(details.active_activities.iter().map(|activity| activity.activity_id.clone()));
        for incident in &details.incidents {
            if let Some(activity) = incident.failed_activity_id.clone().or_else(|| incident.activity_id.clone()) {
                *highlights.incidents.entry(activity).or_default() += 1;
            }
        }
        highlights.completed.extend(details.activity_history.iter().filter(|item| item.completed_count > 0).map(|item| item.activity_id.clone()));
        if let Some(diagram) = &self.diagram {
            diagram.update(cx, |diagram, cx| diagram.update(&details.diagram, highlights, cx));
            if let Some(variables) = &self.variables {
                variables.update(cx, |variables, cx| variables.set_variables(details.variables.clone(), cx));
            }
            self.details = Some(details);
            self.error = None;
            return;
        }
        let variables = cx.new(|cx| VariablesView::new(self.connection.clone(), self.instance_id.clone(), details.variables.clone(), window, cx));
        self._variables_subscription = Some(cx.subscribe_in(&variables, window, |this, _, event: &VariablesChanged, window, cx| {
            this.notice = Some(format!("Переменная {} обновлена.", event.0).into());
            this.reload(window, cx);
        }));
        self.variables = Some(variables);
        let diagram = cx.new(|_| BpmnView::new(details.diagram.clone(), highlights));
        self._diagram_subscription = Some(cx.subscribe_in(&diagram, window, |this, _, event: &SelectNode, window, cx| {
            this.selected_node = Some(event.0.clone());
            let teleportable = this.details.as_ref().is_some_and(|details| details.diagram.teleport_targets().any(|node| node.id == event.0));
            if teleportable && this.can_teleport() {
                this.open_teleport(Some(event.0.clone()), window, cx);
            }
            cx.notify();
        }));
        self.diagram = Some(diagram);
        self.details = Some(details);
        self.error = None;
    }

    fn open_teleport(&mut self, target: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(details) = &self.details else { return };
        let connection = self.connection.clone();
        let teleport = cx.new(|cx| TeleportView::new(connection, details, target, window, cx));
        self._teleport_subscription = Some(cx.subscribe_in(&teleport, window, |this, _, event: &TeleportEvent, window, cx| {
            this.teleport = None;
            if matches!(event, TeleportEvent::Done) {
                this.notice = Some("Телепорт выполнен, состояние заявки обновлено.".into());
                this.reload(window, cx);
            }
            cx.notify();
        }));
        self.teleport = Some(teleport);
        cx.notify();
    }

    pub(crate) fn ask(&mut self, operation: Operation, cx: &mut Context<Self>) {
        self.pending = Some(operation);
        cx.notify();
    }

    fn confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(operation) = self.pending.take() else { return };
        let success = operation.success();
        let (connection, instance_id) = (self.connection.clone(), self.instance_id.clone());
        let request = services::request(cx, move |api| operation.run(api, connection, instance_id));
        self.error = None;
        self.operation = Some(cx.spawn_in(window, async move |this, cx| {
            let result = request.await;
            this.update_in(cx, |this, window, cx| {
                this.operation = None;
                match result {
                    Ok(()) => {
                        this.notice = Some(success.into());
                        this.reload(window, cx);
                    }
                    Err(error) => this.error = Some(error),
                }
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    fn copy(&mut self, text: String, notice: &'static str, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.notice = Some(notice.into());
        cx.notify();
    }

    fn cockpit_url(&self) -> Option<String> {
        let base = self.connection.cockpit_url.trim().trim_end_matches('/');
        (!base.is_empty()).then(|| format!("{base}/default/#/process-instance/{}/runtime", self.instance_id))
    }

    // Markdown-отчёт о состоянии заявки для обращения или чата.
    fn report(&self) -> String {
        let Some(details) = &self.details else { return String::new() };
        let instance = &details.instance;
        let mut lines = vec![
            format!("# Заявка {}", instance.business_key.as_deref().unwrap_or(&instance.id)),
            format!("- Подключение: {} ({})", self.connection.name, self.connection.rest_url),
            format!("- Процесс: {} · {}", instance.definition_key, instance.definition_id),
            format!("- ID заявки: {}", instance.id),
            format!("- Состояние: {}", if instance.ended { "завершена" } else if instance.suspended { "приостановлена" } else { "запущена" }),
            String::new(),
            "## Текущие позиции".into(),
        ];
        lines.extend(details.active_activities.iter().map(|a| format!("- {} ({})", a.activity_name.as_deref().unwrap_or(&a.activity_id), a.activity_id)));
        lines.push(String::new());
        lines.push(format!("## Инциденты ({})", details.incidents.len()));
        for incident in &details.incidents {
            lines.push(format!(
                "- {} · {} · {}",
                incident.incident_type,
                incident.failed_activity_id.as_deref().or(incident.activity_id.as_deref()).unwrap_or("—"),
                incident.timestamp.as_deref().unwrap_or("")
            ));
            if let Some(message) = &incident.message {
                lines.push(format!("  ```\n  {}\n  ```", message.replace('\n', "\n  ")));
            }
        }
        lines.push(String::new());
        lines.push("## Переменные".into());
        lines.extend(details.variables.iter().map(|v| {
            let value: String = v.value.chars().take(200).collect();
            format!("- {} ({}): {value}", v.name, v.value_type)
        }));
        lines.join("\n")
    }

    fn render_confirm(&self, operation: &Operation, cx: &mut Context<Self>) -> impl IntoElement {
        let production = self.connection.environment == Environment::Production;
        div().absolute().inset_0().flex().items_center().justify_center().bg(black().opacity(0.45)).child(
            v_flex()
                .id("confirm")
                .w(px(480.))
                .p_5()
                .gap_4()
                .rounded_lg()
                .border_1()
                .border_color(cx.theme().border)
                .bg(cx.theme().popover)
                .shadow_2xl()
                .occlude()
                .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child("Подтвердите действие"))
                .child(div().child(operation.question()))
                .when(production, |this| this.child(ui::pill("Это прод", Tone::Danger, cx)))
                .child(
                    h_flex()
                        .justify_end()
                        .gap_2()
                        .child(Button::new("confirm-cancel").label("Отмена").ghost().on_click(cx.listener(|this, _, _, cx| {
                            this.pending = None;
                            cx.notify();
                        })))
                        .child(
                            Button::new("confirm-run")
                                .label("Выполнить")
                                .map(|button| if production { button.danger() } else { button.primary() })
                                .on_click(cx.listener(|this, _, window, cx| this.confirm(window, cx))),
                        ),
                ),
        )
    }

    fn can_teleport(&self) -> bool {
        self.details.as_ref().is_some_and(|details| !details.instance.ended && details.active_activities.iter().any(|activity| activity.cancellable))
    }

    fn render_side(&self, details: &ProcessInstanceDetails, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self.selected_node.as_deref().and_then(|id| details.diagram.node(id));
        let can_teleport = self.can_teleport();
        v_flex()
            .id("side")
            .w(px(340.))
            .h_full()
            .flex_none()
            .gap_3()
            .overflow_y_scroll()
            .child(match selected {
                Some(node) => {
                    let history = details.activity_history.iter().find(|item| item.activity_id == node.id);
                    let target = node.id.clone();
                    let teleportable = details.diagram.teleport_targets().any(|candidate| candidate.id == node.id);
                    ui::section(div().font_weight(FontWeight::SEMIBOLD).truncate().child(node.name.clone().unwrap_or_else(|| node.id.clone())), cx).flex_none().child(
                        v_flex()
                            .p_4()
                            .gap_2()
                            .text_sm()
                            .child(div().text_color(cx.theme().muted_foreground).child(format!("{} · {}", node.node_type, node.id)))
                            .children(history.map(|item| format!("выполнено {} · активно {} · отменено {}", item.completed_count, item.active_count, item.canceled_count)))
                            .children(node.topic.clone().map(|topic| format!("топик: {topic}")))
                            .when(teleportable, |this| {
                                this.child(
                                    Button::new("teleport-here")
                                        .label("Телепортировать сюда")
                                        .primary()
                                        .small()
                                        .disabled(!can_teleport)
                                        .on_click(cx.listener(move |this, _, window, cx| this.open_teleport(Some(target.clone()), window, cx))),
                                )
                            }),
                    )
                }
                None => ui::section(div().text_sm().text_color(cx.theme().muted_foreground).child("Нажмите кубик на схеме"), cx).flex_none(),
            })
            .child(
                ui::section(
                    h_flex().w_full().child(div().flex_1().font_weight(FontWeight::SEMIBOLD).child("Текущее состояние")).child(ui::counter(details.active_activities.len(), cx)),
                    cx,
                )
                .flex_none()
                .child(v_flex().children(details.active_activities.iter().map(|activity| {
                    v_flex()
                        .px_4()
                        .py_3()
                        .gap_1p5()
                        .border_b_1()
                        .border_color(cx.theme().border)
                        .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(activity.activity_name.clone().unwrap_or_else(|| activity.activity_id.clone())))
                        .child(
                            h_flex()
                                .gap_1p5()
                                .flex_wrap()
                                .child(ui::pill("активен", Tone::Info, cx))
                                .when(!activity.incident_ids.is_empty(), |this| this.child(ui::pill("инцидент", Tone::Danger, cx)))
                                .child(div().text_xs().text_color(cx.theme().muted_foreground).child(activity.activity_type.clone())),
                        )
                }))),
            )
            .child(
                ui::section(
                    h_flex().w_full().child(div().flex_1().font_weight(FontWeight::SEMIBOLD).child("Инциденты")).child(ui::counter(details.incidents.len(), cx)),
                    cx,
                )
                .flex_none()
                .child(v_flex().p_3().gap_2().children(details.incidents.iter().map(|incident| {
                    v_flex()
                        .p_3()
                        .gap_1()
                        .rounded_md()
                        .bg(cx.theme().danger.opacity(0.08))
                        .border_l_2()
                        .border_color(cx.theme().danger)
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(cx.theme().danger)
                                .child(incident.failed_activity_id.clone().or_else(|| incident.activity_id.clone()).unwrap_or_default()),
                        )
                        .child(div().text_sm().child(incident.message.clone().unwrap_or_else(|| "Без сообщения".into())))
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(format!("{} · {}", model::incident_type_label(&incident.incident_type), model::short_time(incident.timestamp.as_deref()))),
                        )
                })))
                .when(details.incidents.is_empty(), |this| this.child(div().p_4().text_sm().text_color(cx.theme().muted_foreground).child("Инцидентов нет"))),
            )
    }

    fn render_timeline(&self, details: &ProcessInstanceDetails, cx: &App) -> impl IntoElement {
        let head = h_flex()
            .w_full()
            .child(div().flex_1().font_weight(FontWeight::SEMIBOLD).child("Хронология прохождения"))
            .child(div().text_sm().text_color(cx.theme().muted_foreground).child(format!("{} событий · History API", details.activity_timeline.len())));
        let columns = h_flex()
            .flex_none()
            .px_4()
            .py_2()
            .gap_4()
            .text_sm()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(cx.theme().muted_foreground)
            .border_b_1()
            .border_color(cx.theme().border)
            .child(div().w(px(150.)).child("Начало"))
            .child(div().flex_1().child("Кубик"))
            .child(div().w(px(150.)).child("Тип"))
            .child(div().w(px(130.)).text_right().child("Длительность"))
            .child(div().w(px(110.)).text_right().child("Статус"));
        ui::section(head, cx).flex_1().child(columns).child(div().id("timeline").flex_1().overflow_y_scroll().children(details.activity_timeline.iter().map(|entry| {
            let status = if entry.canceled {
                ui::pill("отменён", Tone::Warning, cx)
            } else if entry.end_time.is_none() {
                ui::pill("активен", Tone::Info, cx)
            } else {
                ui::pill("выполнен", Tone::Success, cx)
            };
            h_flex()
                .px_4()
                .py_3()
                .gap_4()
                .border_b_1()
                .border_color(cx.theme().border)
                .text_sm()
                .child(div().w(px(150.)).flex_none().font_family(crate::theme::MONO).text_xs().child(model::short_time(entry.start_time.as_deref())))
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .child(div().truncate().font_weight(FontWeight::MEDIUM).child(entry.activity_name.clone().unwrap_or_else(|| entry.activity_id.clone())))
                        .child(div().truncate().text_xs().text_color(cx.theme().muted_foreground).child(entry.activity_id.clone())),
                )
                .child(div().w(px(150.)).flex_none().truncate().text_color(cx.theme().muted_foreground).child(entry.activity_type.clone()))
                .child(div().w(px(130.)).flex_none().text_right().child(entry.duration_millis.map(format_millis).unwrap_or_else(|| "—".into())))
                .child(div().w(px(110.)).flex_none().flex().justify_end().child(status))
        })))
    }

    fn render_metadata(&self, details: &ProcessInstanceDetails, cx: &App) -> impl IntoElement {
        ui::section(div().font_weight(FontWeight::SEMIBOLD).child("Метаданные заявки"), cx).flex_1().child(
            div().id("metadata").flex_1().overflow_y_scroll().children(details.metadata.iter().map(|field| {
                h_flex()
                    .px_4()
                    .py_2p5()
                    .gap_4()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .text_sm()
                    .child(div().w(px(240.)).flex_none().text_color(cx.theme().muted_foreground).child(field.label.clone()))
                    .child(div().flex_1().min_w_0().font_family(crate::theme::MONO).child(field.value.clone()))
            })),
        )
    }
}

fn format_millis(millis: i64) -> String {
    if millis < 1_000 {
        format!("{millis} мс")
    } else if millis < 60_000 {
        format!("{:.1} с", millis as f64 / 1_000.)
    } else {
        model::format_duration(millis)
    }
}

impl Render for InstanceView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let can_teleport = self.can_teleport();
        let cockpit = self.cockpit_url();
        let link = deep_link::process_instance_deep_link(&self.connection, &self.instance_id);
        let suspended = self.details.as_ref().filter(|details| !details.instance.ended).map(|details| details.instance.suspended);
        let header = h_flex()
            .gap_3()
            .child(Button::new("back").icon(IconName::ChevronLeft).ghost().small().on_click(cx.listener(|_, _, _, cx| cx.emit(Navigate::Back))))
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_0p5()
                    .child(
                        h_flex()
                            .gap_2()
                            .child({
                                let key = self.details.as_ref().and_then(|details| details.instance.business_key.clone());
                                let copy_key = key.clone();
                                div()
                                    .id("business-key")
                                    .text_lg()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .cursor_pointer()
                                    .hover(|style| style.text_color(cx.theme().link))
                                    .child(key.unwrap_or_else(|| "Заявка".into()))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if let Some(key) = copy_key.clone() {
                                            this.copy(key, "Бизнес-ключ скопирован.", cx);
                                        }
                                    }))
                            })
                            .when_some(self.details.as_ref(), |this, details| {
                                this.child(ui::instance_state(details.instance.suspended, details.instance.ended, cx))
                                    .when(!details.incidents.is_empty(), |this| this.child(ui::pill(format!("{} инц.", details.incidents.len()), Tone::Danger, cx)))
                            }),
                    )
                    .child(
                        div().text_xs().font_family(crate::theme::MONO).text_color(cx.theme().muted_foreground).child(format!(
                            "{} · {}",
                            self.details.as_ref().map(|details| details.instance.definition_key.clone()).unwrap_or_default(),
                            self.instance_id
                        )),
                    ),
            )
            .child(
                Button::new("reload")
                    .icon(IconName::RefreshCw)
                    .ghost()
                    .small()
                    .tooltip("Обновить")
                    .loading(self.load.is_some())
                    .on_click(cx.listener(|this, _, window, cx| this.reload(window, cx))),
            )
            .child(
                Button::new("cockpit")
                    .icon(IconName::ExternalLink)
                    .ghost()
                    .small()
                    .tooltip("Открыть в Cockpit")
                    .disabled(cockpit.is_none())
                    .on_click(move |_, _, cx| {
                        if let Some(url) = &cockpit {
                            cx.open_url(url);
                        }
                    }),
            )
            .child(
                Button::new("copy-link")
                    .icon(IconName::Copy)
                    .ghost()
                    .small()
                    .tooltip("Скопировать ссылку camunda-support://")
                    .disabled(link.is_none())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(link) = link.clone() {
                            this.copy(link, "Ссылка на заявку скопирована, по клику она откроется в приложении.", cx);
                        }
                    })),
            )
            .child(
                Button::new("copy-report")
                    .label("Отчёт")
                    .ghost()
                    .small()
                    .tooltip("Скопировать отчёт в Markdown")
                    .disabled(self.details.is_none())
                    .on_click(cx.listener(|this, _, _, cx| {
                        let report = this.report();
                        this.copy(report, "Отчёт по заявке скопирован в Markdown.", cx);
                    })),
            )
            .when_some(suspended, |this, suspended| {
                this.child(
                    Button::new("suspend")
                        .label(if suspended { "Активировать" } else { "Приостановить" })
                        .small()
                        .disabled(self.operation.is_some())
                        .on_click(cx.listener(move |this, _, _, cx| this.ask(Operation::Suspend(!suspended), cx))),
                )
            })
            .child(
                Button::new("teleport")
                    .label("Телепорт")
                    .primary()
                    .small()
                    .disabled(!can_teleport)
                    .on_click(cx.listener(|this, _, window, cx| this.open_teleport(None, window, cx))),
            );

        let body = match (&self.details, &self.diagram) {
            (Some(details), Some(diagram)) => match self.tab {
                0 => h_flex()
                    .flex_1()
                    .min_h_0()
                    .items_start()
                    .gap_4()
                    .child(div().size_full().border_1().border_color(cx.theme().border).rounded_md().overflow_hidden().child(diagram.clone()))
                    .child(self.render_side(details, cx))
                    .into_any_element(),
                1 => self.render_timeline(details, cx).into_any_element(),
                2 => operations::render_diagnostics(details, self.operation.is_some(), cx).into_any_element(),
                3 => self.variables.clone().map(|view| view.into_any_element()).unwrap_or_else(|| div().into_any_element()),
                _ => self.render_metadata(details, cx).into_any_element(),
            },
            _ => div().p_6().text_color(cx.theme().muted_foreground).child("Загружаем заявку…").into_any_element(),
        };

        v_flex()
            .relative()
            .size_full()
            .p_5()
            .gap_4()
            .child(header)
            .child(
                TabBar::new("instance-tabs")
                    .underline()
                    .selected_index(self.tab)
                    .children(TABS)
                    .on_click(cx.listener(|this, index: &usize, _, cx| {
                        this.tab = *index;
                        cx.notify();
                    })),
            )
            .when_some(self.notice.clone(), |this, notice| {
                this.child(div().px_4().py_2().rounded_md().bg(cx.theme().success.opacity(0.12)).text_color(cx.theme().success).child(notice))
            })
            .when_some(self.error.clone(), |this, error| {
                this.child(div().px_4().py_2().rounded_md().bg(cx.theme().danger.opacity(0.1)).text_color(cx.theme().danger).child(error))
            })
            .child(body)
            .when_some(self.pending.clone(), |this, operation| this.child(deferred(self.render_confirm(&operation, cx))))
            .when_some(self.teleport.clone(), |this, teleport| {
                this.child(deferred(
                    div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .p_6()
                        .bg(black().opacity(0.45))
                        .child(teleport),
                ))
            })
    }
}
