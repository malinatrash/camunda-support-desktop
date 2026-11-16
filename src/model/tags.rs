use gpui_kit::ParentElement;
use gpui_kit::component::Sizable;
use gpui_kit::component::tag::Tag;

pub fn state_tag(suspended: bool, ended: bool) -> Tag {
    let (tag, label) = if ended {
        (Tag::secondary(), "завершена")
    } else if suspended {
        (Tag::warning(), "приостановлена")
    } else {
        (Tag::success(), "в работе")
    };
    tag.small().rounded_full().child(label)
}
