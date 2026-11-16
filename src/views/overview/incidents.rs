use gpui_kit::component::accordion::Accordion;
use gpui_kit::component::hover_card::HoverCard;
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme, Sizable, h_flex, v_flex};
use gpui_kit::*;

use super::OverviewView;
use crate::model::{self, IncidentGroup, Navigate};

const CARD_IDS: usize = 10;

impl OverviewView {
    // Группы инцидентов: раскрытая группа показывает заявки.
    pub(super) fn render_incidents(&self, cx: &mut Context<Self>) -> Accordion {
        let keys: Vec<SharedString> = self.incident_groups.iter().map(|group| group.key.clone()).collect();
        let accordion = Accordion::new("incident-groups").h_auto().on_toggle_click(cx.listener(move |this, open: &[usize], _, cx| {
            this.expanded_group = open.first().and_then(|ix| keys.get(*ix)).cloned();
            cx.notify();
        }));
        self.incident_groups.iter().fold(accordion, |accordion, group| {
            let open = self.expanded_group.as_ref() == Some(&group.key);
            let links: Vec<_> = group.instance_ids.iter().take(50).map(|id| instance_link(group, id, cx)).collect();
            accordion.item(|item| {
                item.open(open)
                    .hover(|style| style.bg(cx.theme().table_hover))
                    .title(group_title(group, cx))
                    .content_style(StyleRefinement::default().pl(px(72.)))
                    .children(links)
            })
        })
    }
}

fn group_title(group: &IncidentGroup, cx: &App) -> Div {
    let meta = tr!(
        "overview.group_meta",
        process = group.process_key,
        kind = model::incident_type_label(&group.incident_type),
        count = group.instance_ids.len(),
        time = model::short_time(group.latest.as_deref())
    );
    let more = group.instance_ids.len().saturating_sub(CARD_IDS);
    let card = v_flex()
        .gap_2()
        .text_sm()
        .child(div().font_weight(FontWeight::SEMIBOLD).child(group.activity.clone()))
        .child(div().whitespace_normal().child(group.signature.clone()))
        .child(
            v_flex()
                .font_family(crate::platform::theme::mono(cx))
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .children(group.instance_ids.iter().take(CARD_IDS).cloned())
                .children((more > 0).then(|| tr!("overview.more", count = more))),
        );
    let signature = HoverCard::new(SharedString::from(format!("card-{}", group.key)))
        .w(px(420.))
        .trigger_style(StyleRefinement::default().w_full().min_w_0())
        .trigger(div().truncate().text_sm().font_weight(FontWeight::NORMAL).child(group.signature.clone()))
        .child(card);
    h_flex()
        .flex_1()
        .items_start()
        .gap_3()
        .min_w_0()
        .child(
            div()
                .min_w(px(44.))
                .flex_none()
                .flex()
                .justify_end()
                .child(Tag::danger().small().rounded_full().child(group.count.to_string())),
        )
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .child(div().truncate().child(group.activity.clone()))
                .child(signature)
                .child(div().truncate().text_xs().font_weight(FontWeight::NORMAL).text_color(cx.theme().muted_foreground).child(meta)),
        )
}

fn instance_link(group: &IncidentGroup, id: &str, cx: &mut Context<OverviewView>) -> Stateful<Div> {
    let target = id.to_string();
    div()
        .id(SharedString::from(format!("{}-{id}", group.key)))
        .py_1()
        .text_sm()
        .cursor_pointer()
        .hover(|style| style.underline())
        .child(target.clone())
        .on_click(cx.listener(move |_, _, _, cx| cx.emit(Navigate::Instance(target.clone()))))
}
