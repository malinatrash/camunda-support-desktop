use gpui_kit::ParentElement;
use gpui_kit::component::Sizable;
use gpui_kit::component::tag::Tag;

pub fn state_tag(suspended: bool, ended: bool) -> Tag {
    let (tag, label) = if ended {
        (Tag::secondary(), tr!("model.state.ended"))
    } else if suspended {
        (Tag::warning(), tr!("model.state.suspended"))
    } else {
        (Tag::success(), tr!("model.state.active"))
    };
    tag.small().rounded_full().child(label)
}
