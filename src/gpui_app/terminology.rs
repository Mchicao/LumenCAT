use super::{LumenCatApp, PendingOp, components::*, theme::Theme};
use crate::{model::*, worker::Task};
use gpui::{prelude::*, *};

impl LumenCatApp {
    pub(super) fn refresh_term_bases(&mut self) {
        self.send(Task::TermBases, PendingOp::Operation);
    }

    pub(super) fn create_term_base(&mut self) {
        if !self.opened || self.is_busy() || self.is_dirty() {
            return;
        }
        self.send(
            Task::CreateTermBase(self.term_base_name_input.text.clone()),
            PendingOp::Operation,
        );
    }

    pub(super) fn add_term_concept(&mut self) {
        let Some(base_id) = self.selected_term_base else {
            return;
        };
        if !self.opened || self.is_busy() || self.is_dirty() {
            return;
        }
        let (sl, tl) = self.document_languages();
        let concept = NewTermConcept {
            base_id,
            domain: String::new(),
            notes: self.term_notes_input.text.clone(),
            provenance: "Entrada manual GPUI".into(),
            expressions: vec![
                TermExpression {
                    language: sl,
                    text: self.term_source_input.text.clone(),
                    status: TermStatus::Preferred,
                    case_sensitive: self.term_case_sensitive,
                },
                TermExpression {
                    language: tl,
                    text: self.term_target_input.text.clone(),
                    status: self.term_target_status,
                    case_sensitive: self.term_case_sensitive,
                },
            ],
        };
        self.send(
            Task::AddTermConcept(concept, Cancellation::default()),
            PendingOp::Operation,
        );
    }

    pub(super) fn render_terminology(&self, entity: Entity<Self>) -> impl IntoElement {
        let ready = self.opened && !self.is_busy() && !self.is_dirty();
        let create = entity.clone();
        let toggle_form = entity.clone();
        let status = entity.clone();
        let case = entity.clone();
        let add = entity.clone();
        let (sl, tl) = self.document_languages();
        div().flex().flex_col().gap_2()
            .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child("Bases terminológicas"))
            .child(input_field(super::input::InputField::TermBaseName, &self.term_base_name_input, entity.clone()))
            .child(quick_button("Crear base", ButtonVariant::Secondary, ready && !self.term_base_name_input.text.trim().is_empty(), move |_, _, cx| {
                create.update(cx, |this, cx| { this.create_term_base(); cx.notify(); });
            }))
            .children(self.term_bases.iter().map(|base| {
                let id = base.id;
                let enabled = base.enabled;
                let select = entity.clone();
                let toggle = entity.clone();
                div().flex().gap_1().items_center()
                    .child(quick_button(base.name.clone(), if self.selected_term_base == Some(id) { ButtonVariant::Primary } else { ButtonVariant::Ghost }, ready, move |_, _, cx| {
                        select.update(cx, |this, cx| { this.selected_term_base = Some(id); cx.notify(); });
                    }))
                    .child(quick_button(if enabled { "Desactivar" } else { "Activar" }, ButtonVariant::Ghost, ready, move |_, _, cx| {
                        toggle.update(cx, |this, cx| { this.send(Task::SetTermBaseEnabled(id, !enabled), PendingOp::Operation); cx.notify(); });
                    }))
            }))
            .when(self.term_bases.is_empty(), |d| d.child(div().text_xs().text_color(Theme::text_secondary()).child("Crea una base para guardar términos del cliente. No se añaden a la memoria TM.")))
            .child(quick_button(if self.show_term_form { "Ocultar formulario" } else { "Añadir concepto" }, ButtonVariant::Secondary, self.selected_term_base.is_some(), move |_, _, cx| {
                toggle_form.update(cx, |this, cx| { this.show_term_form = !this.show_term_form; cx.notify(); });
            }))
            .when(self.show_term_form, |d| d.child(
                div().flex().flex_col().gap_2().py_2()
                    .child(div().text_xs().child(format!("Nuevo concepto · {sl} → {tl}")))
                    .child(input_field(super::input::InputField::TermSource, &self.term_source_input, entity.clone()))
                    .child(input_field(super::input::InputField::TermTarget, &self.term_target_input, entity.clone()))
                    .child(quick_button(match self.term_target_status { TermStatus::Preferred => "Destino: preferido", TermStatus::Allowed => "Destino: permitido", TermStatus::Forbidden => "Destino: prohibido" }, ButtonVariant::Secondary, true, move |_, _, cx| {
                        status.update(cx, |this, cx| { this.term_target_status = match this.term_target_status { TermStatus::Preferred => TermStatus::Allowed, TermStatus::Allowed => TermStatus::Forbidden, TermStatus::Forbidden => TermStatus::Preferred }; cx.notify(); });
                    }))
                    .child(quick_button(if self.term_case_sensitive { "Mayúsculas: distinguir" } else { "Mayúsculas: ignorar" }, ButtonVariant::Secondary, true, move |_, _, cx| {
                        case.update(cx, |this, cx| { this.term_case_sensitive = !this.term_case_sensitive; cx.notify(); });
                    }))
                    .child(input_field(super::input::InputField::TermNotes, &self.term_notes_input, entity.clone()))
                    .child(quick_button("Guardar concepto", ButtonVariant::Primary, ready && !self.term_source_input.text.trim().is_empty() && !self.term_target_input.text.trim().is_empty(), move |_, _, cx| {
                        add.update(cx, |this, cx| { this.add_term_concept(); cx.notify(); });
                    }))
            ))
            .child(div().mt_3().text_sm().font_weight(FontWeight::SEMIBOLD).child("Reconocidos en el origen"))
            .child(div().text_xs().text_color(Theme::text_secondary()).child("Frases completas; sin flexión ni segmentación para lenguas sin espacios."))
            .children(if let Some(result) = &self.terminology_result {
                if result.matches.is_empty() {
                    vec![div().text_xs().text_color(Theme::text_secondary()).child("Sin términos reconocidos en las bases activas para este par.").into_any_element()]
                } else {
                    result.matches.iter().map(|found| {
                        div().py_2().border_b_1().border_color(Theme::border_subtle()).flex().flex_col().gap_1()
                            .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(found.source.clone()))
                            .child(div().text_xs().text_color(Theme::text_secondary()).child(format!("{} · concepto {} · {}", found.base_name, found.concept_id, found.provenance)))
                            .children(found.targets.iter().map(|target| div().text_xs().text_color(if target.status == TermStatus::Forbidden { Theme::rose() } else { Theme::text_primary() }).child(format!("{} · {}", target.text, match target.status { TermStatus::Preferred => "preferido", TermStatus::Allowed => "permitido", TermStatus::Forbidden => "prohibido" }))))
                            .when(!found.notes.is_empty(), |d| d.child(div().text_xs().text_color(Theme::text_secondary()).child(found.notes.clone())))
                            .children(result.issues.iter().filter(|i| i.concept_id == found.concept_id && i.source_range == found.source_range).map(|i| div().text_xs().text_color(Theme::amber()).child(i.message.clone())))
                            .into_any_element()
                    }).collect()
                }
            } else {
                vec![div().text_xs().text_color(Theme::text_secondary()).child("Reconocimiento pendiente o no evaluado.").into_any_element()]
            })
    }
}
