use camunda_core::deep_link;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::toolbar::Toolbar;
use gpui_kit::component::{Disableable, IconName};
use gpui_kit::*;

use super::InstanceView;
use super::operations::Operation;

impl InstanceView {
    // Панель действий заявки: обновить, Cockpit, ссылка, отчёт, приостановка и телепорт.
    pub(super) fn render_actions(&self, cx: &mut Context<Self>) -> Toolbar {
        let cockpit = self.cockpit_url();
        let link = deep_link::process_instance_deep_link(&self.connection, &self.instance_id);
        let suspended = self.details.as_ref().filter(|details| !details.instance.ended).map(|details| details.instance.suspended);
        Toolbar::new("instance-actions")
            .child(
                Button::new("reload")
                    .icon(IconName::RefreshCw)
                    .ghost()
                    .tooltip("Обновить")
                    .loading(self.load.is_some())
                    .on_click(cx.listener(|this, _, window, cx| this.reload(window, cx))),
            )
            .child(
                Button::new("cockpit")
                    .icon(IconName::ExternalLink)
                    .ghost()
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
                    .tooltip("Скопировать отчёт в Markdown")
                    .disabled(self.details.is_none())
                    .on_click(cx.listener(|this, _, _, cx| {
                        let report = this.report();
                        this.copy(report, "Отчёт по заявке скопирован в Markdown.", cx);
                    })),
            )
            .children(suspended.map(|suspended| {
                Button::new("suspend")
                    .label(if suspended { "Активировать" } else { "Приостановить" })
                    .outline()
                    .disabled(self.operation.is_some())
                    .on_click(cx.listener(move |this, _, window, cx| this.ask(Operation::Suspend(!suspended), window, cx)))
            }))
            .child(
                Button::new("teleport")
                    .label("Телепорт")
                    .primary()
                    .disabled(!self.can_teleport())
                    .on_click(cx.listener(|this, _, window, cx| this.open_teleport(None, window, cx))),
            )
    }

    pub(super) fn copy(&mut self, text: String, notice: &'static str, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.notice = Some(notice.into());
        cx.notify();
    }

    fn cockpit_url(&self) -> Option<String> {
        let base = self.connection.cockpit_url.trim().trim_end_matches('/');
        (!base.is_empty()).then(|| format!("{base}/default/#/process-instance/{}/runtime", self.instance_id))
    }
}
