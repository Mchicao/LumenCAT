use super::theme::Theme;
use crate::model::SegmentState;
use gpui::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    Primary,
    Success,
    Secondary,
    Ghost,
    Danger,
}

pub fn custom_button<F>(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    variant: ButtonVariant,
    enabled: bool,
    on_click_handler: F,
) -> Stateful<Div>
where
    F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
{
    use gpui::prelude::*;
    let id = id.into();
    let label = label.into();
    let mut btn = div()
        .id(id.clone())
        .accessibility_id(id)
        .role(Role::Button)
        .aria_label(label.clone())
        .a11y_synthetic_children(move |tree| {
            if !enabled {
                tree.parent_node().set_disabled();
            }
        })
        .when(enabled, |d| d.focusable().tab_stop(true))
        .focus_visible(|s| s.border_color(Theme::sky()))
        .flex()
        .items_center()
        .justify_center()
        .px_3()
        .py_1p5()
        .rounded_md()
        .text_sm()
        .font_weight(FontWeight::MEDIUM)
        .cursor_pointer();

    if !enabled {
        return btn
            .bg(Theme::bg_subtle())
            .text_color(Theme::text_muted())
            .border_1()
            .border_color(Theme::border_subtle())
            .cursor_default()
            .child(label);
    }

    match variant {
        ButtonVariant::Primary => {
            btn = btn
                .bg(Theme::sky())
                .text_color(rgb(0x0a101d))
                .font_weight(FontWeight::SEMIBOLD)
                .hover(|s| s.bg(rgb(0x7dd3fc)));
        }
        ButtonVariant::Success => {
            btn = btn
                .bg(Theme::emerald())
                .text_color(rgb(0x062817))
                .font_weight(FontWeight::SEMIBOLD)
                .hover(|s| s.bg(rgb(0x34d399)));
        }
        ButtonVariant::Secondary => {
            btn = btn
                .bg(Theme::bg_subtle())
                .text_color(Theme::text_primary())
                .border_1()
                .border_color(Theme::border_medium())
                .hover(|s| s.bg(Theme::bg_hover()).border_color(Theme::sky()));
        }
        ButtonVariant::Ghost => {
            btn = btn
                .bg(gpui::transparent_black())
                .text_color(Theme::text_secondary())
                .hover(|s| s.bg(Theme::bg_hover()).text_color(Theme::text_primary()));
        }
        ButtonVariant::Danger => {
            btn = btn
                .bg(Theme::rose_bg())
                .text_color(Theme::rose())
                .border_1()
                .border_color(Theme::rose())
                .hover(|s| s.bg(Theme::rose()).text_color(Theme::text_primary()));
        }
    }

    btn.on_click(on_click_handler).child(label)
}

pub fn history_button<F>(redo: bool, enabled: bool, handler: F) -> impl IntoElement
where
    F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
{
    use gpui::prelude::*;
    div()
        .id(if redo { "redo" } else { "undo" })
        .accessibility_id(if redo { "redo" } else { "undo" })
        .role(Role::Button)
        .aria_label(if redo { "Rehacer" } else { "Deshacer" })
        .aria_keyshortcuts(if redo { "Control+Y" } else { "Control+Z" })
        .a11y_synthetic_children(move |tree| {
            if !enabled {
                tree.parent_node().set_disabled();
            }
        })
        .size(px(30.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .when(enabled, |d| {
            d.focusable()
                .tab_stop(true)
                .cursor_pointer()
                .hover(|s| s.bg(Theme::bg_hover()))
                .on_click(handler)
        })
        .tooltip(move |_, cx| cx.new(move |_| HistoryTooltip(redo)).into())
        .child(
            canvas(
                |_, _, _| (),
                move |bounds, _, window, _| {
                    let at = |x: f32, y: f32| {
                        bounds.origin + point(px(if redo { 18. - x } else { x }), px(y))
                    };
                    let mut path = PathBuilder::stroke(px(1.7));
                    path.move_to(at(7., 3.));
                    path.line_to(at(3., 7.));
                    path.line_to(at(7., 11.));
                    path.move_to(at(3., 7.));
                    path.line_to(at(10., 7.));
                    path.curve_to(at(15., 12.), at(15., 7.));
                    path.curve_to(at(10., 16.), at(15., 16.));
                    path.line_to(at(7., 16.));
                    if let Ok(path) = path.build() {
                        window.paint_path(
                            path,
                            if enabled {
                                Theme::text_secondary()
                            } else {
                                Theme::text_muted()
                            },
                        );
                    }
                },
            )
            .size(px(18.)),
        )
}

struct HistoryTooltip(bool);

impl Render for HistoryTooltip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py_1()
            .rounded_md()
            .bg(Theme::bg_surface())
            .text_color(Theme::text_primary())
            .text_xs()
            .child(if self.0 {
                "Rehacer · Ctrl+Y"
            } else {
                "Deshacer · Ctrl+Z"
            })
    }
}

pub fn status_badge(state: SegmentState, locked: bool) -> Div {
    let base = div()
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .py_0p5()
        .rounded_sm()
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD);

    if locked {
        return base
            .bg(Theme::slate_bg())
            .text_color(Theme::slate())
            .border_1()
            .border_color(Theme::slate())
            .child("🔒 Locked");
    }

    match state {
        SegmentState::Confirmed => base
            .bg(Theme::emerald_bg())
            .text_color(Theme::emerald())
            .border_1()
            .border_color(Theme::emerald())
            .child("✓ Confirmed"),
        SegmentState::Draft => base
            .bg(Theme::amber_bg())
            .text_color(Theme::amber())
            .border_1()
            .border_color(Theme::amber())
            .child("✎ Draft"),
    }
}

pub fn tm_badge(score: f64, exact: bool) -> Div {
    let base = div()
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .py_0p5()
        .rounded_sm()
        .text_xs()
        .font_weight(FontWeight::BOLD);

    if exact || score >= 99.9 {
        base.bg(Theme::emerald_bg())
            .text_color(Theme::emerald())
            .border_1()
            .border_color(Theme::emerald())
            .child(format!("{:.0}% EXACT", score))
    } else {
        base.bg(Theme::violet_bg())
            .text_color(Theme::violet())
            .border_1()
            .border_color(Theme::violet())
            .child(format!("{:.0}% FUZZY", score))
    }
}

pub fn format_badge(fmt: &str) -> Div {
    div()
        .px_1p5()
        .py_0p5()
        .rounded_sm()
        .text_xs()
        .font_weight(FontWeight::BOLD)
        .bg(Theme::bg_subtle())
        .text_color(Theme::sky())
        .border_1()
        .border_color(Theme::border_subtle())
        .child(fmt.to_uppercase())
}

pub fn progress_bar(translated: usize, total: usize) -> Div {
    let ratio = if total == 0 {
        0.0
    } else {
        (translated as f32 / total as f32).clamp(0.0, 1.0)
    };

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .flex()
                .justify_between()
                .text_xs()
                .text_color(Theme::text_secondary())
                .child(format!("{translated} / {total} segments"))
                .child(format!("{:.0}%", ratio * 100.0)),
        )
        .child(
            div()
                .w_full()
                .h(px(6.))
                .rounded_full()
                .bg(Theme::bg_subtle())
                .overflow_hidden()
                .child(
                    div()
                        .h_full()
                        .w(relative(ratio))
                        .rounded_full()
                        .bg(if ratio >= 1.0 {
                            Theme::emerald()
                        } else {
                            Theme::sky()
                        }),
                ),
        )
}

#[derive(Clone)]
pub struct InputModel {
    pub text: String,
    pub cursor: usize,
    pub selected_all: bool,
    pub marked: Option<std::ops::Range<usize>>,
    pub placeholder: String,
    pub focus_handle: FocusHandle,
}

impl InputModel {
    pub fn new(placeholder: impl Into<String>, cx: &mut App) -> Self {
        Self {
            text: String::new(),
            cursor: 0,
            selected_all: false,
            marked: None,
            placeholder: placeholder.into(),
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.cursor = self.text.len();
        self.selected_all = false;
        self.marked = None;
    }

    pub fn insert(&mut self, value: &str) {
        if self.selected_all {
            self.text.clear();
            self.cursor = 0;
            self.selected_all = false;
        }
        self.text.insert_str(self.cursor, value);
        self.cursor += value.len();
    }

    pub fn handle_key(&mut self, event: &KeyDownEvent, cx: &mut App) -> bool {
        let ks = &event.keystroke;

        if ks.modifiers.control && ks.key == "a" {
            self.cursor = self.text.len();
            self.selected_all = true;
            return true;
        }

        if ks.modifiers.control && ks.key == "c" {
            cx.write_to_clipboard(ClipboardItem::new_string(self.text.clone()));
            return true;
        }

        if ks.modifiers.control && ks.key == "v" {
            if let Some(clip) = cx.read_from_clipboard().and_then(|item| item.text()) {
                let clean = clip.replace("\r\n", "\n");
                self.insert(&clean);
                return true;
            }
            return false;
        }

        if self.selected_all && matches!(ks.key.as_str(), "backspace" | "delete") {
            self.set_text("");
            return true;
        }
        if matches!(ks.key.as_str(), "left" | "right" | "home" | "end") {
            self.selected_all = false;
        }
        match ks.key.as_str() {
            "backspace" => {
                if self.cursor > 0 && !self.text.is_empty() {
                    let prev = self.text[..self.cursor]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    self.text.drain(prev..self.cursor);
                    self.cursor = prev;
                    return true;
                }
            }
            "delete" => {
                if self.cursor < self.text.len() {
                    let next = self.text[self.cursor..]
                        .char_indices()
                        .nth(1)
                        .map(|(i, _)| self.cursor + i)
                        .unwrap_or(self.text.len());
                    self.text.drain(self.cursor..next);
                    return true;
                }
            }
            "left" => {
                if self.cursor > 0 {
                    self.cursor = self.text[..self.cursor]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    return true;
                }
            }
            "right" => {
                if self.cursor < self.text.len() {
                    self.cursor = self.text[self.cursor..]
                        .char_indices()
                        .nth(1)
                        .map(|(i, _)| self.cursor + i)
                        .unwrap_or(self.text.len());
                    return true;
                }
            }
            "home" => {
                self.cursor = 0;
                return true;
            }
            "enter" => {
                self.insert("\n");
                return true;
            }
            "end" => {
                self.cursor = self.text.len();
                return true;
            }
            _ => {
                if !ks.modifiers.control
                    && !ks.modifiers.alt
                    && let Some(ch) = &ks.key_char
                {
                    self.insert(ch);
                    return true;
                }
            }
        }
        false
    }
}

pub fn inline_text(text: &str) -> Div {
    div().flex().flex_wrap().items_center().gap_1().children(
        crate::editing::parts(text).into_iter().map(|(part, tag)| {
            let d = div().child(part.to_string());
            if tag {
                d.px_1()
                    .rounded_sm()
                    .text_xs()
                    .bg(Theme::violet_bg())
                    .text_color(Theme::violet())
            } else {
                d
            }
        }),
    )
}

pub fn input_field(
    field: super::input::InputField,
    input: &InputModel,
    entity: Entity<super::LumenCatApp>,
) -> impl IntoElement {
    use gpui::prelude::*;
    let focus = input.focus_handle.clone();
    super::input::accessible_input(field, input, entity.clone(), true)
        .relative()
        .child(super::input::native_input(
            input.focus_handle.clone(),
            entity,
        ))
        .track_focus(&input.focus_handle)
        .flex_1()
        .min_w(px(70.))
        .h(px(28.))
        .px_2()
        .flex()
        .items_center()
        .bg(Theme::bg_card())
        .rounded_md()
        .border_1()
        .border_color(Theme::border_subtle())
        .focus(|d| d.border_color(Theme::sky()))
        .text_xs()
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            focus.focus(window, cx)
        })
        .child(if input.text.is_empty() {
            input.placeholder.clone()
        } else {
            input.text.clone()
        })
}
