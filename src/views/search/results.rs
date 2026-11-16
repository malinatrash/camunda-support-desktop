use camunda_core::models::ProcessInstanceSummary;
use gpui_kit::component::empty::{Empty, EmptyHeader, EmptyTitle};
use gpui_kit::component::skeleton::Skeleton;
use gpui_kit::component::table::DataTable;
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme, Sizable, h_flex, v_flex};
use gpui_kit::*;

use super::{Mode, SearchView};

impl SearchView {
    pub(super) fn render_results(&self, cx: &mut Context<Self>) -> AnyElement {
        if self.search.is_some() {
            return v_flex().gap_3().children((0..6).map(|_| Skeleton::new().h_10().rounded_md())).into_any_element();
        }
        let Some(count) = self.results else { return div().into_any_element() };
        if count == 0 {
            let title = match self.mode {
                Mode::Key => "Заявки не найдены. Проверьте бизнес-ключ или ID.",
                Mode::Variable => "Активных заявок с таким значением не найдено.",
            };
            return Empty::new().flex_1().header(EmptyHeader::new().title(EmptyTitle::new().child(title))).into_any_element();
        }
        v_flex()
            .flex_1()
            .min_h_0()
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .child(
                h_flex()
                    .px_4()
                    .py_3()
                    .gap_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(div().font_weight(FontWeight::SEMIBOLD).child("Найденные заявки"))
                    .child(Tag::secondary().small().rounded_full().child(count.to_string()))
                    .child(div().flex_1())
                    .child(div().text_sm().text_color(cx.theme().muted_foreground).child("Двойной клик открывает заявку")),
            )
            .child(div().flex_1().min_h_0().child(DataTable::new(&self.table).stripe(false).bordered(false)))
            .into_any_element()
    }

    pub(super) fn start(
        &mut self,
        request: impl Future<Output = Result<Vec<ProcessInstanceSummary>, String>> + 'static,
        cx: &mut Context<Self>,
    ) {
        self.error = None;
        self.search = Some(cx.spawn(async move |this, cx| {
            let result = request.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(rows) => {
                        this.results = Some(rows.len());
                        this.table.update(cx, |table, cx| {
                            table.delegate_mut().rows = rows;
                            table.refresh(cx);
                        });
                    }
                    Err(error) => this.error = Some(error),
                }
                this.search = None;
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    pub(super) fn select_mode(&mut self, mode: Mode, cx: &mut Context<Self>) {
        if self.mode == mode {
            return;
        }
        self.mode = mode;
        self.error = None;
        self.results = None;
        self.table.update(cx, |table, cx| {
            table.delegate_mut().rows.clear();
            table.refresh(cx);
        });
        cx.notify();
    }
}
