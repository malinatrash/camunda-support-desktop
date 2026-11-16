use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::form::{field, v_form};
use gpui_kit::component::h_flex;
use gpui_kit::component::input::Input;
use gpui_kit::*;

use super::SearchView;
use crate::platform::services;

impl SearchView {
    pub(super) fn render_key_form(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_form().child(
            field().label(tr!("search.key.label")).description(tr!("search.key.description")).child(
                h_flex().gap_2().child(div().flex_1().child(Input::new(&self.key_query))).child(
                    Button::new("search-key")
                        .label(tr!("search.find"))
                        .primary()
                        .loading(self.search.is_some())
                        .on_click(cx.listener(|this, _, _, cx| this.search_by_key(cx))),
                ),
            ),
        )
    }

    pub(super) fn search_by_key(&mut self, cx: &mut Context<Self>) {
        let query = self.key_query.read(cx).value().trim().to_string();
        if query.is_empty() {
            self.error = Some(tr!("search.key.required").into());
            cx.notify();
            return;
        }
        let connection = self.connection.clone();
        let request = services::request(cx, move |api| async move { api.search_instances_by_business_key(&connection, &query).await });
        self.start(request, cx);
    }
}
