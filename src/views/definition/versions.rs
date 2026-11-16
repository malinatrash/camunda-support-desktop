use gpui_kit::component::Sizable;
use gpui_kit::component::tab::TabBar;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::DefinitionView;

impl DefinitionView {
    // Переключатель «Все версии» и конкретных версий процесса.
    pub(super) fn render_versions(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = match &self.version_id {
            Some(id) => self.versions.iter().position(|version| &version.id == id).map(|ix| ix + 1),
            None => Some(0),
        };
        let labels = self.versions.iter().map(|version| format!("v{}", version.version));
        TabBar::new("definition-versions")
            .segmented()
            .small()
            .when_some(selected, |this, ix| this.selected_index(ix))
            .child("Все версии")
            .children(labels)
            .on_click(cx.listener(|this, index: &usize, _, cx| {
                this.version_id = index.checked_sub(1).and_then(|ix| this.versions.get(ix)).map(|version| version.id.clone());
                this.reload(cx);
            }))
    }
}
