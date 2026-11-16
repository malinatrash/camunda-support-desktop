use camunda_core::models::ProcessDefinitionSummary;
use gpui_kit::component::description_list::DescriptionList;
use gpui_kit::component::hover_card::HoverCard;
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme, Sizable, h_flex};
use gpui_kit::*;

use super::OverviewView;
use crate::model::{self, Navigate, ProcessGroup};

pub(super) const NUMBER_WIDTH: Pixels = px(80.);
pub(super) const TRAILING: Pixels = px(36.);

impl OverviewView {
    // Строки раскрытого процесса: «Все версии» и каждая версия.
    pub(super) fn render_versions(&self, group: &ProcessGroup, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let all_id = group.latest.id.clone();
        let all_title = div().child(tr!("overview.all_versions")).into_any_element();
        let all = version_row(SharedString::from(format!("{}-all", group.key)), all_title, None, group.instances, None, cx)
            .on_click(cx.listener(move |_, _, _, cx| cx.emit(Navigate::Definition { id: all_id.clone(), all_versions: true })));
        let versions = group.versions.iter().map(|version| {
            let id = version.id.clone();
            let meta = format!(
                "{}{}{}",
                model::short_time(version.deployment_time.as_deref()),
                version.version_tag.as_ref().map(|tag| format!(" · {tag}")).unwrap_or_default(),
                if version.suspended { tr!("overview.paused") } else { SharedString::default() }
            );
            let title = definition_card(version.id.clone(), version, div().child(format!("v{}", version.version))).into_any_element();
            version_row(SharedString::from(version.id.clone()), title, Some(meta.into()), version.instances, Some(version.incidents), cx)
                .on_click(cx.listener(move |_, _, _, cx| cx.emit(Navigate::Definition { id: id.clone(), all_versions: false })))
                .into_any_element()
        });
        std::iter::once(all.into_any_element()).chain(versions).collect()
    }
}

// Карточка версии процесса при наведении.
pub(super) fn definition_card(id: String, definition: &ProcessDefinitionSummary, trigger: impl IntoElement + 'static) -> HoverCard {
    let list = DescriptionList::new()
        .columns(1)
        .label_width(px(110.))
        .item(tr!("overview.card.key"), definition.key.clone(), 1)
        .item(
            tr!("overview.card.version"),
            format!("v{}{}", definition.version, if definition.suspended { tr!("overview.paused") } else { SharedString::default() }),
            1,
        )
        .item(tr!("overview.card.deployed"), model::short_time(definition.deployment_time.as_deref()), 1)
        .item(tr!("overview.card.tag"), definition.version_tag.clone().unwrap_or_else(|| "—".into()), 1)
        .item(tr!("overview.card.instances"), model::grouped(definition.instances as i64), 1)
        .item(tr!("overview.card.incidents"), model::grouped(definition.incidents as i64), 1);
    HoverCard::new(SharedString::from(format!("card-{id}")))
        .w(px(360.))
        .trigger_style(StyleRefinement::default().w_full().min_w_0())
        .trigger(trigger)
        .child(list)
}

// Количество инцидентов: красная метка или серый ноль.
pub(super) fn incidents_cell(count: i32, cx: &App) -> AnyElement {
    if count > 0 {
        Tag::danger().small().rounded_full().child(count.to_string()).into_any_element()
    } else {
        div().text_color(cx.theme().muted_foreground).child("0").into_any_element()
    }
}

fn version_row(
    id: SharedString,
    title: AnyElement,
    meta: Option<SharedString>,
    instances: i32,
    incidents: Option<i32>,
    cx: &App,
) -> Stateful<Div> {
    h_flex()
        .id(id)
        .pl_3()
        .pr(TRAILING)
        .py_2()
        .gap_3()
        .text_sm()
        .font_weight(FontWeight::NORMAL)
        .cursor_pointer()
        .hover(|style| style.bg(cx.theme().table_hover))
        .child(div().min_w(px(90.)).flex_none().font_weight(FontWeight::MEDIUM).child(title))
        .child(div().flex_1().min_w_0().truncate().text_color(cx.theme().muted_foreground).children(meta))
        .child(div().min_w(NUMBER_WIDTH).text_right().child(model::grouped(instances as i64)))
        .child(div().min_w(NUMBER_WIDTH).flex().justify_end().children(incidents.map(|count| incidents_cell(count, cx))))
}
