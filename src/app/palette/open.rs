use gpui_kit::component::WindowExt as _;
use gpui_kit::*;

use super::{CommandPalette, PaletteEvent};
use crate::app::AppView;
use crate::model::Navigate;
use crate::platform::theme;

impl AppView {
    // ⌘K: палитра в диалоге GPUI Kit, повторное нажатие закрывает.
    pub(in crate::app) fn toggle_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) {
            window.close_dialog(cx);
            return;
        }
        let palette = cx.new(|cx| CommandPalette::new(self.connection.clone(), window, cx));
        self._palette_subscription = Some(cx.subscribe_in(&palette, window, |this, _, event: &PaletteEvent, window, cx| {
            window.close_dialog(cx);
            match event {
                PaletteEvent::Open(id) => {
                    this.pages.clear();
                    this.open_page(Navigate::Instance(id.clone()), window, cx);
                }
                PaletteEvent::Go(section) => this.go(*section, window, cx),
            }
            this.focus.focus(window, cx);
        }));
        window.open_dialog(cx, move |dialog, _, cx| {
            dialog.bg(theme::surface(cx)).w(px(640.)).p_0().close_button(false).child(palette.clone())
        });
    }
}
