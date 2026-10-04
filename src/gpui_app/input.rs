use super::{LumenCatApp, components::InputModel};
use crate::model::Origin;
use gpui::*;
use std::ops::Range;

#[derive(Clone, Copy)]
pub enum InputField {
    Target,
    Search,
    Replacement,
}

impl InputField {
    fn id(self) -> &'static str {
        match self {
            Self::Target => "target-editor",
            Self::Search => "search-query",
            Self::Replacement => "replacement-text",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Target => "Destino",
            Self::Search => "Buscar en origen y destino",
            Self::Replacement => "Texto de reemplazo",
        }
    }
}

pub fn accessible_input(
    field: InputField,
    input: &InputModel,
    entity: Entity<LumenCatApp>,
    enabled: bool,
) -> Stateful<Div> {
    use gpui::prelude::*;
    div()
        .id(field.id())
        .accessibility_id(field.id())
        .role(Role::TextInput)
        .aria_label(field.label())
        .aria_value(input.text.clone())
        .aria_placeholder(input.placeholder.clone())
        .tab_stop(enabled)
        .a11y_synthetic_children(move |tree| {
            if !enabled {
                tree.parent_node().set_read_only();
            }
        })
        .when(enabled, |d| {
            d.on_a11y_action(AccessibleAction::SetValue, move |data, _, cx| {
                if let Some(accesskit::ActionData::Value(value)) = data {
                    entity.update(cx, |this, cx| {
                        this.set_accessible_value(field, value);
                        cx.notify();
                    });
                }
            })
        })
}

pub fn native_input(focus: FocusHandle, entity: Entity<LumenCatApp>) -> impl IntoElement {
    use gpui::prelude::*;
    canvas(
        |_, _, _| (),
        move |bounds, _, window, cx| {
            window.handle_input(&focus, ElementInputHandler::new(bounds, entity.clone()), cx);
        },
    )
    .absolute()
    .size_full()
}

impl LumenCatApp {
    fn set_accessible_value(&mut self, field: InputField, value: &str) {
        if value.len() > 1_048_576 {
            self.message = "El texto excede el límite de 1 MiB".into();
            self.message_error = true;
            return;
        }
        match field {
            InputField::Target => {
                if self.active_draft.as_ref().is_none_or(|a| a.segment.locked) {
                    return;
                }
                self.target_input.set_text(value);
                self.on_target_text_changed(Origin::Human);
            }
            InputField::Search => self.search_input.set_text(value),
            InputField::Replacement => self.replacement_input.set_text(value),
        }
    }

    fn focused_input(&self, window: &Window) -> &InputModel {
        if self.search_input.focus_handle.is_focused(window) {
            &self.search_input
        } else if self.replacement_input.focus_handle.is_focused(window) {
            &self.replacement_input
        } else {
            &self.target_input
        }
    }
    fn focused_input_mut(&mut self, window: &Window) -> &mut InputModel {
        if self.search_input.focus_handle.is_focused(window) {
            &mut self.search_input
        } else if self.replacement_input.focus_handle.is_focused(window) {
            &mut self.replacement_input
        } else {
            &mut self.target_input
        }
    }
    fn replace_native(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        window: &Window,
    ) -> Option<Range<usize>> {
        let target = !self.search_input.focus_handle.is_focused(window)
            && !self.replacement_input.focus_handle.is_focused(window);
        if target && self.active_draft.as_ref().is_none_or(|a| a.segment.locked) {
            return None;
        }
        let input = self.focused_input_mut(window);
        let range = range
            .map(|r| {
                crate::editing::utf16_offset(&input.text, r.start)
                    ..crate::editing::utf16_offset(&input.text, r.end)
            })
            .or(input.marked.clone())
            .unwrap_or({
                if input.selected_all {
                    0..input.text.len()
                } else {
                    input.cursor..input.cursor
                }
            });
        input.text.replace_range(range.clone(), text);
        input.cursor = range.start + text.len();
        input.selected_all = false;
        input.marked = None;
        if target {
            self.on_target_text_changed(Origin::Human);
        }
        Some(range.start..range.start + text.len())
    }
}

impl EntityInputHandler for LumenCatApp {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        actual: &mut Option<Range<usize>>,
        window: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let input = self.focused_input(window);
        let bytes = crate::editing::utf16_offset(&input.text, range.start)
            ..crate::editing::utf16_offset(&input.text, range.end);
        *actual = Some(
            input.text[..bytes.start].encode_utf16().count()
                ..input.text[..bytes.end].encode_utf16().count(),
        );
        Some(input.text[bytes].to_string())
    }
    fn selected_text_range(
        &mut self,
        _: bool,
        window: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let input = self.focused_input(window);
        let cursor = input.text[..input.cursor].encode_utf16().count();
        Some(UTF16Selection {
            range: if input.selected_all {
                0..input.text.encode_utf16().count()
            } else {
                cursor..cursor
            },
            reversed: false,
        })
    }
    fn marked_text_range(
        &self,
        window: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        let input = self.focused_input(window);
        input.marked.as_ref().map(|r| {
            input.text[..r.start].encode_utf16().count()..input.text[..r.end].encode_utf16().count()
        })
    }
    fn unmark_text(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.focused_input_mut(window).marked = None;
        cx.notify();
    }
    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.replace_native(range, text, window);
        cx.notify();
    }
    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        selected: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(inserted) = self.replace_native(range, text, window) {
            let input = self.focused_input_mut(window);
            if !text.is_empty() {
                input.marked = Some(inserted.clone());
            }
            if let Some(selected) = selected {
                input.cursor = inserted.start + crate::editing::utf16_offset(text, selected.end);
            }
        }
        cx.notify();
    }
    fn bounds_for_range(
        &mut self,
        _: Range<usize>,
        bounds: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        Some(bounds)
    }
    fn character_index_for_point(
        &mut self,
        _: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        None
    }
}
