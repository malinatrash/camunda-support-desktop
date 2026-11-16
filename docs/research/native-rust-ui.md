# GPUI with GPUI Kit wins, Slint is the fallback

As of November 2026, the best choice for a native Camunda Support without a WebView is **GPUI (the Zed engine) with GPUI Kit from Longbridge (formerly gpui-component)**. It is the only stack that renders directly through Metal on macOS with its own AppKit layer, was designed for a 120 Hz frame budget (8.33 ms), and ships everything we need out of the box: a virtualized DataTable, VirtualList, Input/Textarea with IME, Select/Combobox, Dialog, Tabs, a ready command palette, themes, clipboard, Dock integration and a canvas with Bezier paths for BPMN.

The price: the stack is pre-1.0, breaking changes land every 1-3 weeks, official documentation is thin, and upstream snapshots on crates.io are published in practice by one person at Longbridge.

**Fallback: Slint 1.18.** It is the only candidate with a stable semver release line, mature IME, working accessibility and a virtualized ListView. On the other hand, it has no immediate-mode canvas for BPMN, async with tokio needs a separate runtime, and the Royalty-free license requires showing AboutSlint.

Iced, egui and Freya are technically viable, but each fails one key requirement: virtualization and accessibility, a declarative model, or resilience to a single maintainer leaving. Makepad, Floem, Xilem, Vizia and Dioxus Native/Blitz are not ready for a production tool in 2026. No framework has independent 120 Hz frame measurements, so the decision should be confirmed with a 1-2 day prototype.

## Only GPUI was built for 120 Hz and owns the platform layer

The main requirement (120 Hz, instant response) separates the candidates by architecture, not by widget count. GPUI draws each primitive type with its own instanced shader (SDF rectangles, shadows, glyphs from a GPU atlas). Text is shaped by the OS through CoreText, and **the whole frame must fit in 8.33 ms** ([Zed blog](https://zed.dev/blog/videogame)). macOS runs on Metal, Windows has used DirectX 11 and DirectWrite since October 2025 ([Zed docs](https://zed.dev/docs/windows.md)), and Linux moved from Blade to wgpu in February 2026 after freezes on NVIDIA and Wayland ([PR #46758](https://github.com/zed-industries/zed/pull/46758)). Dropping wgpu on macOS and Windows was deliberate: a reviewer measured **~10 MB of memory for an empty window with the native renderer versus ~100 MB with wgpu** (same PR).

There are no published frame time numbers for upstream. The most concrete data comes from the Longbridge fork GPUI Fast: CPU time per frame when scrolling a sidebar dropped from 5.95 to 0.79 ms, and with a quote stream from 5.06 to 1.35 ms. So upstream fits the 120 Hz budget with a small margin, while GPUI Fast, enabled by a single feature in GPUI Kit, gives a multiple of that margin but is experimental ([gpui-fast](https://github.com/longbridge/gpui-fast)).

The second architectural advantage is that GPUI does not depend on winit. egui, Iced, Slint, Floem, Vizia and Xilem are built on winit, and all of them inherit its long-standing `NSTextInputClient` problems on macOS and limited handling of file and URL open events ([winit #3617](https://github.com/rust-windowing/winit/issues/3617), [Tritium](https://tritium.legal/blog/desktop)). For Cyrillic layouts this barely matters, since Cyrillic is typed without marked text. It does explain why text behavior in GPUI is closer to native.

Performance of the other candidates:

- **Slint** has created only visible ListView items since 1.16, and 1.18 reduced redundant redraws: shadows were redrawn every frame and the visibility check took exponential time ([Slint CHANGELOG](https://github.com/slint-ui/slint/blob/master/CHANGELOG.md)). Its partial redraw and dirty-property tracking suit 120 Hz well.
- **Iced** 0.14 got reactive rendering, but the frame clock and smooth scrolling are only in master and have not been released ([iced #3527](https://github.com/iced-rs/iced/pull/3527), [#3479](https://github.com/iced-rs/iced/pull/3479)).
- **egui** rebuilds the UI every frame (1-2 ms), and after wgpu became the default, hello world takes **129-176 MB of RAM instead of 53 MB** ([egui #7761](https://github.com/emilk/egui/issues/7761)).
- **Dioxus** says plainly that Native is "not focused on performance" ([Dioxus 0.7](https://dioxuslabs.com/blog/release-070/)).
- **Xilem**'s Vello renderer is called "roughly beta quality" by its own developers ([Linebender 2026 Q1](https://linebender.org/blog/tmil-25/)).

## Ready-made widgets: GPUI Kit covers the list, Slint almost

| Framework | UI model | macOS rendering | Virtual table | Palette / modals / tabs | Canvas for BPMN | A11y | Status, Nov 2026 | License |
|---|---|---|---|---|---|---|---|---|
| **GPUI + GPUI Kit** | View-Entity + `render()`, `cx.notify()` | Metal, own AppKit layer | DataTable, VirtualList | Command, Dialog, Tab, all ready | `canvas`, PathBuilder (Bezier, arcs, dash) | AccessKit since 2026, incomplete | gpui-kit 0.9.0, pre-1.0, release every 1-3 weeks | Apache-2.0 |
| **Slint** | `.slint` DSL, reactive properties | Skia/FemtoVG (Vello experimental) | StandardTableView on ListView, text cells only | Popup, TabWidget, palette must be built | Declarative `Path` only, or render to texture | AccessKit, mature | 1.18.1, stable semver | GPLv3 / Royalty-free / commercial |
| **Iced** | Elm (update/view) | wgpu | None in core, manual | No palette, stack/float overlays | Best `Canvas` + `Cache` | None upstream | 0.14 (Dec 2025), 0.15 only in git | MIT |
| **egui** | Immediate mode | wgpu | `TableBody::rows` | Modal, Popup, palette must be built | `Scene` with zoom and pan | AccessKit, mature | 0.37, monthly releases | MIT/Apache |
| **Freya** | Own hooks (React-like) | Skia/Metal | Table + VirtualScrollView | Popup, Select, floating tab | Skia canvas, draggable_canvas | AccessKit, partial | 0.4.3, 0.5-rc | MIT |
| **Vizia** | Signals, CSS | Skia | VirtualTable, VirtualTreeTable | TabView, Popup, ComboBox | Skia view | AccessKit | 0.4 (Apr 2026), 1 maintainer | MIT |
| **Makepad** | Retained + script DSL | Own shader pipeline | PortalList | Modal, Tabs | Shaders/SDF | Own layer, new | 1.0 on crates.io, 2.0 only in main | MIT |
| **Floem** | Signals | vger/wgpu | virtual_list | dropdown, tab, overlay | canvas | None | 0.2.0 (2024), git only | MIT |
| **Xilem/Masonry** | Elm/SwiftUI-like | Vello | virtual_scroll, no table | No modals or tabs | Canvas | AccessKit | 0.4.0 (Oct 2025), "experimental" | Apache-2.0 |
| **Dioxus Native/Blitz** | rsx!, signals | Vello/Skia via HTML/CSS engine | No virtualization | HTML forms, partial | Inline SVG | AccessKit | beta, 0.8-alpha | MIT/Apache |

Table sources: [gpui-kit](https://github.com/longbridge/gpui-kit), [Slint backends](https://docs.slint.dev/latest/docs/slint/guide/backends-and-renderers/backends_and_renderers/), [iced widgets](https://github.com/iced-rs/iced/tree/master/widget/src), [egui 0.37](https://github.com/emilk/egui/releases/tag/0.37.0), [Freya 0.4](https://freyaui.dev/posts/0.4/), [vizia views](https://github.com/vizia/vizia/tree/main/crates/vizia_core/src/views), [makepad](https://github.com/makepad/makepad), [floem #1050](https://github.com/lapce/floem/issues/1050), [xilem](https://github.com/linebender/xilem), [blitz](https://github.com/DioxusLabs/blitz).

GPUI Kit has **more than 75 components**. Among them is a DataTable with virtual scrolling, fixed columns, resizable columns, sorting and cell selection. The authors claim it handles "hundreds of thousands of rows" and has been used in the commercial Longbridge Pro since day one ([gpui-kit README](https://github.com/longbridge/gpui-kit)). Version 0.9.0 added localization of components, a grid role for tables and named rows in the command palette ([v0.9.0](https://github.com/longbridge/gpui-kit/releases/tag/v0.9.0)). The table also has known weak spots, all in open issues: Shift/Cmd row selection needs a table fork (#2795), Tab does not move focus out of DataTable (#3788), and there is a request for faster scrolling (#2109) ([issues](https://github.com/longbridge/gpui-kit/issues?q=is%3Aopen+table)). The first limitation is noticeable for instance and incident lists, but it can be worked around.

Slint is thinner in the details. StandardTableView is built on a virtualized ListView, but rows are described as `[[StandardListViewItem]]`, so cells are text only ([tableview.slint](https://github.com/slint-ui/slint/blob/master/internal/compiler/widgets/fluent/tableview.slint)). Status badges and colored cells would require building the table on ListView by hand, and the command palette would have to be assembled from PopupWindow, LineEdit and a list.

Iced is worse on the main requirement: core has no virtual list, and when asked about one the maintainer answered "we don't have a `list` widget" ([iced #3429](https://github.com/iced-rs/iced/issues/3429)).

On paper, Vizia fits best: signals, CSS themes, VirtualTable and VirtualTreeTable out of the box. But it has one main developer, releases come once a year, and IME was "basic" in 2025 and tested only on Windows ([vizia #566](https://github.com/vizia/vizia/pull/566)).

## BPMN canvas and tokio plug into GPUI without workarounds

For BPMN, GPUI has the `canvas(prepaint, paint)` element, `window.paint_quad` for rounded task rectangles and `PathBuilder` with `line_to`, `cubic_bezier_to`, `arc_to`, `add_polygon`, dashing and transforms (scale, translate, rotate). Tessellation goes through lyon ([path_builder.rs](https://github.com/zed-industries/zed/blob/main/crates/gpui/src/path_builder.rs), [canvas.rs](https://github.com/zed-industries/zed/blob/main/crates/gpui/src/elements/canvas.rs)). Gateway diamonds, event circles and flows with arrowheads map directly onto these primitives. Zoom and pan use a custom matrix, and hit testing uses custom geometry. Tessellation runs on the CPU, so paths should be cached while the scale is unchanged.

No framework has a ready canvas with zoom and pan. The closest are egui `Scene` ([egui 0.37](https://github.com/emilk/egui/releases/tag/0.37.0)) and Iced `Canvas` with a geometry `Cache` ([iced 0.14](https://github.com/iced-rs/iced/releases/tag/0.14.0)). Slint has no immediate-mode canvas at all: the diagram would have to be built as a `for` loop over `Path` and `Rectangle` inside a scalable container with a `TouchArea` per element, or rendered into a wgpu texture ([Slint CHANGELOG](https://github.com/slint-ui/slint/blob/master/CHANGELOG.md)). That works for diagrams with hundreds of elements, but it is the most awkward part of Slint for this use case.

Async is handled like this. GPUI has its own executor, and reqwest outside a tokio context fails with "there is no reactor running" ([reqwest #1961](https://github.com/seanmonstar/reqwest/discussions/1961)). Zed solves this with the `gpui_tokio` crate, about 100 lines: `init_from_handle(cx, handle)` accepts your own runtime, `Tokio::spawn(cx, fut)` returns a GPUI `Task`, and dropping it cancels the tokio task ([gpui_tokio.rs](https://github.com/zed-industries/zed/blob/main/crates/gpui_tokio/src/gpui_tokio.rs)). The crate is not on crates.io, but there is a community port, `gpui-tokio-bridge` ([docs.rs](https://docs.rs/gpui-tokio-bridge)). The flow is simple: a Camunda REST request runs on tokio, and the result reaches the Entity through `this.update(cx, ...)` and `cx.notify()`. Reactivity is manual, and redraws happen per view, without signals.

Iced is the most convenient with tokio: it is an executor feature plus `Task::perform` ([iced Cargo.toml](https://github.com/iced-rs/iced/blob/master/Cargo.toml)). In Slint, `#[tokio::main]` is not recommended: the runtime needs its own thread, and results are passed back through `spawn_local` or `invoke_from_event_loop` ([docs.rs slint](https://docs.rs/slint/latest/slint/fn.spawn_local.html)).

Text and IME in GPUI are shaped natively, and Cyrillic works as expected. Open IME bugs concern Pinyin (#3605) and dead keys (#3456) on macOS ([issues](https://github.com/longbridge/gpui-kit/issues?q=is%3Aopen+IME)), which barely affect Russian layouts. A 2026 survey mentions a GPUI crash when typing again after deleting composition text, and the lack of a stock text field ([daily.dev, survey review](https://daily.dev/posts/a-2026-survey-of-rust-gui-libraries-e93cfc0b7)). The second point does not apply to GPUI Kit, and the first one should be checked in the prototype.

For IME and accessibility the survey puts Slint and egui first. AccessKit appeared in GPUI in 2026 on all three platforms ([_accessibility.rs](https://github.com/zed-industries/zed/blob/main/crates/gpui/src/_accessibility.rs)), but it is not battle-tested yet: DataTable rows had no screen reader names before 0.9.0 ([#3647](https://github.com/longbridge/gpui-kit/issues/3647)). For an internal support tool this is sufficient.

## The GPUI risk is delivery, not technology

The official `gpui` on crates.io stopped at 0.2.2 (October 2025), and the Zed README warns: "There will often be breaking changes" ([crates.io gpui](https://crates.io/crates/gpui), [README](https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md)). In practice Longbridge removed this problem. Weekly upstream snapshots are published as `gpui-pre` (0.3.11 from 2 November 2026), and `gpui-kit` pins them strictly, so the whole stack installs as a single crates.io dependency ([crates.io gpui-pre](https://crates.io/crates/gpui-pre), [deps 0.9.0](https://crates.io/crates/gpui-component/0.9.0/dependencies)).

The downside: minor versions break the API (in 0.9.0 theme structs became `#[non_exhaustive]`), and the snapshots are published by one maintainer. So the version must be pinned and upgraded deliberately. Learning material is limited to the Zed code and Discord, but GPUI Kit has a website, a story gallery, AGENTS.md/CLAUDE.md and AI skills for coding agents ([gpui-kit README](https://github.com/longbridge/gpui-kit)), which partly closes the documentation gap. The Tritium claim that Zed "abandoned" GPUI as open source ([Tritium](https://tritium.legal/blog/desktop)) contradicts the active work in the Zed repository in 2026: `gpui_platform` was split out, Linux moved to wgpu, AccessKit was added.

Data on size and startup is scarce. A hello world on plain GPUI weighed 8.2 MB, and with gpui-component 22 MB after tree-sitter was moved behind features (August 2025) ([discussion #1132](https://github.com/longbridge/gpui-kit/discussions/1132)). A secondary report says that when the chipmunk app moved from Tauri to GPUI, memory dropped from ~500 MB to under 100 MB and startup became "almost instant" ([Reddit snapshot](https://reddit.sentinel-team.org/posts/1u329gm/snapshots/2026-06-12T15%3A08%3A48.007531Z)). This matters here because the previous version of Camunda Support was built on Tauri and Svelte.

Slint is the opposite: compromises in the technology, calm delivery. Four minor releases in 2026 (1.15-1.18) upgrade through `cargo update`, and unstable APIs are hidden behind `unstable-*` features ([crates.io slint](https://crates.io/api/v1/crates/slint/versions)). The Royalty-free 2.0 license allows a proprietary desktop app on one condition: AboutSlint is shown in an About window or on a splash screen ([LicenseRef-Slint-Royalty-free-2.0](https://github.com/slint-ui/slint/blob/master/LICENSES/LicenseRef-Slint-Royalty-free-2.0.md)). That is acceptable for an internal tool, but legal should confirm it.

Iced is formally stable, but releases come once a year and each one breaks the API. COSMIC and Halloy live on git forks of 0.15-dev ([halloy Cargo.toml](https://github.com/squidowl/halloy/blob/main/Cargo.toml)). Upstream Iced has had no accessibility support since 2020 ([iced #552](https://github.com/iced-rs/iced/issues/552)).

The rest are excluded for specific reasons:

- **Floem** has not released to crates.io since November 2024, and 1.0 is, in the maintainer's words, "probably not in the next 2-4 years" ([floem #1050](https://github.com/lapce/floem/issues/1050)).
- **Makepad** is stuck on 1.0 on crates.io (May 2025), while main holds an unreleased rewrite, 2.0, with a new DSL in which `for` does not redraw when the array changes ([splash.md](https://github.com/makepad/makepad/blob/main/splash.md)).
- **Xilem** calls itself "experimental" and has had no release for over a year ([xilem](https://github.com/linebender/xilem)).
- **Blitz** is in beta, has no virtualization and has CSS gaps ([blitz.is/status](https://blitz.is/status)).
- **egui** is mature, well funded by Rerun, and strong on accessibility and IME, but it is immediate mode. Tritium tried to move off it to Slint for the sake of a declarative model and abandoned the migration after 4 weeks and 45% done ([Tritium](https://tritium.legal/blog/desktop)).
- **Freya** 0.4 looks like the best dark horse: own hooks, `freya-query`, tokio examples, a virtual table, Skia/Metal ([Freya 0.4](https://freyaui.dev/posts/0.4/)). But it has a single maintainer and 3.3k stars, so it is riskier than Slint for a production tool.

## Conclusion

The choice is driven not by who has more widgets but by where the risk sits. For GPUI with GPUI Kit the risk is delivery: pre-1.0, weekly snapshots, one publisher. This is handled by pinning the version and upgrading once a month. Technically, the stack covers every requirement and is the only one designed for 120 Hz on Metal.

For Slint the risk is technical: no BPMN canvas, text-only table cells, two-language code with `.slint`. This risk is built into the architecture and will not go away over time. So GPUI goes first, and Slint is the fallback in case Longbridge stops publishing snapshots or the prototype shows unstable IME. If performance can be traded for a declarative model, Slint and Freya share second place overall.

No source gives comparable frame measurements on ProMotion, so a 1-2 day prototype on Apple Silicon is needed before the rewrite. It should include:

- a DataTable with 50k instance rows scrolling at 120 Hz;
- a search field with Cyrillic input, Cmd+C/V and the "delete composition and type again" scenario;
- a BPMN canvas with 300+ elements, zoom and pan;
- a Camunda REST request through `gpui_tokio`.

Measure CPU per frame (including with the `gpui-fast` feature), RSS, binary size and cold start. In parallel, it is worth building the same table in Slint in half a day, so the fallback has its own numbers and not just a reputation.
