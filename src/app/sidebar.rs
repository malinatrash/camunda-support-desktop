use gpui_kit::assets::IconName as Lucide;
use gpui_kit::component::sidebar::{Sidebar, SidebarCollapsible, SidebarFooter, SidebarGroup, SidebarMenu, SidebarMenuItem};
use gpui_kit::component::{ActiveTheme, Sizable, TitleBar, h_flex, v_flex};
use gpui_kit::*;

use super::AppView;
use super::palette::Section;
use crate::platform::theme;
use crate::updates::VERSION;

impl AppView {
    // Боковая панель: подключение, разделы и тема.
    pub(super) fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let dark = cx.theme().is_dark();
        Sidebar::new("sidebar")
            .collapsible(SidebarCollapsible::Offcanvas)
            .collapsed(self.sidebar_collapsed)
            .header(
                v_flex()
                    .w_full()
                    .gap_2()
                    .child(
                        TitleBar::new()
                            .bg(transparent_black())
                            .border_color(transparent_black())
                            .h(px(22.))
                            .child(h_flex().w_full().justify_end().child(self.sidebar_toggle(cx))),
                    )
                    .child(self.render_connection_menu(cx)),
            )
            .child(SidebarGroup::new(tr!("sidebar.monitoring")).child(SidebarMenu::new().children([
                self.section_item(Section::Overview, tr!("sidebar.overview"), Lucide::LayoutDashboard, cx),
                self.section_item(Section::Search, tr!("sidebar.search"), Lucide::Search, cx),
                self.section_item(Section::Incidents, tr!("sidebar.incidents"), Lucide::TriangleAlert, cx),
            ])))
            .child(
                SidebarGroup::new(tr!("sidebar.application")).child(
                    SidebarMenu::new().children([
                        self.section_item(Section::Settings, tr!("sidebar.settings"), Lucide::Settings, cx),
                        SidebarMenuItem::new(if dark { tr!("sidebar.light_theme") } else { tr!("sidebar.dark_theme") })
                            .icon(if dark { Lucide::Sun } else { Lucide::Moon })
                            .on_click(|_, window, cx| theme::toggle(window, cx)),
                    ]),
                ),
            )
            .footer(
                SidebarFooter::new().child(
                    v_flex()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(tr!("app.name"))
                        .child(h_flex().gap_1().child(tr!("sidebar.version", version = VERSION))),
                ),
            )
    }

    fn section_item(&self, section: Section, label: SharedString, icon: Lucide, cx: &mut Context<Self>) -> SidebarMenuItem {
        let view = cx.entity().downgrade();
        SidebarMenuItem::new(label)
            .icon(gpui_kit::component::Icon::new(icon).small())
            .active(self.section == section && self.pages.is_empty())
            .on_click(move |_, window, cx| {
                view.update(cx, |this, cx| this.go(section, window, cx)).ok();
            })
    }
}
