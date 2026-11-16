use camunda_core::models::ProcessVariable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::table::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use gpui_kit::component::{ActiveTheme, Disableable, Sizable, h_flex, v_flex};
use gpui_kit::*;

use super::VariablesView;
use crate::views::instance::hover;
use crate::views::instance::panel::empty;

const READ_ONLY_TYPES: [&str; 4] = ["object", "file", "bytes", "null"];

impl VariablesView {
    // Таблица переменных с копированием значения и правкой изменяемых типов.
    pub(super) fn render_list(&self, visible: Vec<&ProcessVariable>, cx: &mut Context<Self>) -> AnyElement {
        if visible.is_empty() {
            return empty(tr!("instance.variables.empty")).into_any_element();
        }
        let mono = crate::platform::theme::mono(cx);
        let rows = visible.into_iter().map(|variable| {
            let copy_value = variable.value.clone();
            let target = variable.clone();
            let can_edit = !READ_ONLY_TYPES.contains(&variable.value_type.to_lowercase().as_str());
            TableRow::new()
                .child(
                    TableCell::new().col_span(2).child(
                        v_flex()
                            .min_w_0()
                            .child(
                                div().truncate().font_family(mono.clone()).font_weight(FontWeight::SEMIBOLD).child(variable.name.clone()),
                            )
                            .child(div().text_xs().text_color(cx.theme().muted_foreground).child(variable.value_type.clone())),
                    ),
                )
                .child(TableCell::new().col_span(5).child(hover::text_card(
                    SharedString::from(format!("value-{}", variable.name)),
                    div().min_w_0().line_clamp(3).font_family(mono.clone()).text_xs().child(variable.value.clone()),
                    format!("{} · {}", variable.name, variable.value_type),
                    variable.value.clone(),
                )))
                .child(
                    TableCell::new().col_span(2).text_right().child(
                        h_flex()
                            .gap_1()
                            .child(
                                Button::new(SharedString::from(format!("copy-{}", variable.name)))
                                    .label(tr!("instance.variables.copy"))
                                    .ghost()
                                    .small()
                                    .on_click(move |_, _, cx| cx.write_to_clipboard(ClipboardItem::new_string(copy_value.clone()))),
                            )
                            .child(
                                Button::new(SharedString::from(format!("edit-{}", variable.name)))
                                    .label(if can_edit { tr!("instance.variables.edit") } else { tr!("instance.variables.read_only") })
                                    .outline()
                                    .small()
                                    .disabled(!can_edit)
                                    .on_click(cx.listener(move |this, _, window, cx| this.edit(target.clone(), window, cx))),
                            ),
                    ),
                )
        });
        let header = TableRow::new()
            .child(TableHead::new().col_span(2).child(tr!("instance.variables.col_key")))
            .child(TableHead::new().col_span(5).child(tr!("instance.variables.col_value")))
            .child(TableHead::new().col_span(2).child(""));
        Table::new().child(TableHeader::new().child(header)).child(TableBody::new().children(rows)).into_any_element()
    }
}
