use gpui_kit::*;

actions!(camunda_support, [Quit, TogglePalette, OpenOverview, OpenSearch, OpenIncidents, OpenSettings, ToggleSidebar]);

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-k", TogglePalette, None),
        KeyBinding::new("ctrl-k", TogglePalette, None),
        KeyBinding::new("cmd-1", OpenOverview, None),
        KeyBinding::new("cmd-2", OpenSearch, None),
        KeyBinding::new("cmd-3", OpenIncidents, None),
        KeyBinding::new("cmd-,", OpenSettings, None),
        KeyBinding::new("cmd-ctrl-s", ToggleSidebar, None),
    ]);
    cx.on_action(|_: &Quit, cx| cx.quit());
}
