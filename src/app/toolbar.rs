use gpui_kit::assets::IconName as Lucide;
use gpui_kit::component::breadcrumb::{Breadcrumb, BreadcrumbItem};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::kbd::Kbd;
use gpui_kit::component::sidebar::SidebarToggleButton;
use gpui_kit::component::{Sizable, TitleBar, h_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::AppView;
use super::palette::Section;

impl AppView {
    // Верхняя полоса над содержимым: кнопка боковой панели, когда она свёрнута, и поиск ⌘K.
    pub(super) fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        TitleBar::new()
            .when(!self.sidebar_collapsed, |bar| bar.pl_3())
            .bg(crate::platform::theme::surface(cx))
            .border_color(transparent_black())
            .child(
                h_flex()
                    .w_full()
                    .pr_3()
                    .gap_2()
                    .when(self.sidebar_collapsed, |this| this.child(self.sidebar_toggle(cx)))
                    .child(self.render_breadcrumbs(cx))
                    .child(div().flex_1())
                    .child(
                        Button::new("palette-trigger")
                            .ghost()
                            .small()
                            .icon(Lucide::Search)
                            .child(
                                h_flex()
                                    .gap_3()
                                    .child(tr!("shell.find_instance"))
                                    .child(Kbd::new(Keystroke::parse("cmd-k").expect("сочетание"))),
                            )
                            .on_click(cx.listener(|this, _, window, cx| this.toggle_palette(window, cx))),
                    ),
            )
    }

    // Путь: раздел и открытые поверх него процесс и заявка; клик возвращает на уровень.
    fn render_breadcrumbs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section = self.section;
        let root = cx.entity().downgrade();
        let mut crumbs = Breadcrumb::new().child(BreadcrumbItem::new(section_label(section)).on_click(move |_, window, cx| {
            root.update(cx, |this, cx| this.go(section, window, cx)).ok();
        }));
        for (depth, page) in self.pages.iter().enumerate() {
            let view = cx.entity().downgrade();
            crumbs = crumbs.child(BreadcrumbItem::new(page.label()).on_click(move |_, _, cx| {
                view.update(cx, |this, cx| this.truncate_pages(depth + 1, cx)).ok();
            }));
        }
        div().min_w_0().overflow_hidden().child(crumbs)
    }

    pub(super) fn sidebar_toggle(&self, cx: &mut Context<Self>) -> impl IntoElement {
        SidebarToggleButton::new().collapsed(self.sidebar_collapsed).on_click(cx.listener(|this, _, _, cx| this.toggle_sidebar(cx)))
    }

    pub(super) fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.sidebar_collapsed = !self.sidebar_collapsed;
        cx.notify();
    }
}

fn section_label(section: Section) -> SharedString {
    match section {
        Section::Overview => tr!("sidebar.overview"),
        Section::Search => tr!("sidebar.search"),
        Section::Incidents => tr!("sidebar.incidents"),
        Section::Settings => tr!("sidebar.settings"),
    }
}
