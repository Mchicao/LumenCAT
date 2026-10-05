use super::{LumenCatApp, PendingOp, WorkerTask, components::custom_button};
use gpui::component::{WindowExt, button::ButtonVariant};
use gpui::{
    Context, InteractiveElement, ParentElement, StatefulInteractiveElement, Styled, Window, div, px,
};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum ExitState {
    #[default]
    Running,
    Prompt,
    Saving,
    Discarding,
    Closing,
}

impl ExitState {
    fn can_close(self, dirty: bool, busy: bool, saving: bool) -> bool {
        !busy && !saving && (self == Self::Discarding || (self == Self::Saving && !dirty))
    }
}

impl LumenCatApp {
    pub(super) fn request_exit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.exit_state != ExitState::Running || self.native_dialog_open {
            return;
        }
        self.exit_state = ExitState::Prompt;
        self.exit_error = None;
        let entity = cx.entity();
        window.open_dialog(cx, move |dialog, _, cx| {
            let app = entity.read(cx);
            let prompt = app.exit_state == ExitState::Prompt;
            let opened = app.opened;
            let error = app.exit_error.clone();
            let description = if !prompt {
                "Espera a que terminen las operaciones y se cierre el proyecto de forma segura."
            } else if opened {
                "¿Quieres guardar los avances pendientes antes de cerrar LumenCAT?"
            } else {
                "¿Quieres cerrar LumenCAT?"
            };
            let cancel = entity.clone();
            let cancel_button = entity.clone();
            let dismiss = entity.clone();
            let save = entity.clone();
            let discard = entity.clone();
            dialog
                .title(if prompt { "Salir de LumenCAT" } else { "Cerrando LumenCAT…" })
                .w(px(560.))
                .close_button(false)
                .overlay_closable(false)
                .keyboard(prompt)
                .on_cancel(move |_, _, cx| {
                    cancel.update(cx, |this, cx| {
                        if this.exit_state != ExitState::Prompt {
                            return false;
                        }
                        this.exit_state = ExitState::Running;
                        this.exit_error = None;
                        cx.notify();
                        true
                    })
                })
                .on_close(move |_, _, cx| {
                    dismiss.update(cx, |this, cx| {
                        if this.exit_state == ExitState::Prompt {
                            this.exit_state = ExitState::Running;
                            this.exit_error = None;
                            cx.notify();
                        }
                    });
                })
                .child(div().text_sm().child(description))
                .child(div().text_sm().text_color(super::Theme::text_secondary()).child(
                    "El autoguardado se pausa mientras decides. Salir sin guardar descarta solo lo pendiente; no revierte avances ya guardados ni operaciones en curso.",
                ))
                .children(error.map(|error| {
                    div().id("exit-error").role(gpui::Role::Alert)
                        .text_sm().text_color(super::Theme::rose()).child(error)
                }))
                .footer(
                    div().flex().flex_wrap().justify_end().gap_2()
                        .child(custom_button("exit-cancel", "Cancelar", ButtonVariant::Secondary, prompt, move |_, window, cx| {
                            cancel_button.update(cx, |this, cx| {
                                this.exit_state = ExitState::Running;
                                this.exit_error = None;
                                cx.notify();
                            });
                            window.close_dialog(cx);
                        }))
                        .child(custom_button("exit-discard", if opened { "Salir sin guardar" } else { "Salir" }, ButtonVariant::Secondary, prompt, move |_, window, cx| {
                            discard.update(cx, |this, cx| this.choose_exit(false, window, cx));
                        }))
                        .children(opened.then(|| custom_button("exit-save", "Guardar y salir", ButtonVariant::Primary, prompt, move |_, window, cx| {
                            save.update(cx, |this, cx| this.choose_exit(true, window, cx));
                        }))),
                )
        });
        cx.notify();
    }

    fn choose_exit(&mut self, save: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.exit_state != ExitState::Prompt {
            return;
        }
        self.exit_error = None;
        self.navigate_after_save = None;
        self.exit_state = if save {
            ExitState::Saving
        } else {
            ExitState::Discarding
        };
        if save {
            self.save_error = false;
        }
        self.advance_exit();
        self.finish_exit(window, cx);
        cx.notify();
    }

    pub(super) fn advance_exit(&mut self) {
        if self.exit_state == ExitState::Saving {
            self.save(true);
            if self.save_error
                || (self.is_dirty() && self.active_draft.as_ref().is_some_and(|a| !a.saving))
            {
                self.fail_exit(self.message.clone());
                return;
            }
        }
        let saving = self
            .pending
            .values()
            .any(|pending| matches!(pending, PendingOp::Save(_) | PendingOp::Confirm(_)));
        if self
            .exit_state
            .can_close(self.is_dirty(), self.is_busy(), saving)
        {
            if !self.opened || self.send(WorkerTask::Close, PendingOp::Close) {
                self.exit_state = ExitState::Closing;
            } else {
                self.fail_exit(self.message.clone());
            }
        }
    }

    pub(super) fn finish_exit(&self, window: &mut Window, cx: &mut Context<Self>) {
        if self.exit_state == ExitState::Closing && !self.opened {
            window.remove_window();
            cx.quit();
        }
    }

    pub(super) fn fail_exit(&mut self, error: String) {
        if self.exit_state != ExitState::Running {
            self.exit_state = ExitState::Prompt;
            self.exit_error = Some(format!(
                "No se pudo guardar o cerrar: {error}. La ventana sigue abierta y el texto se conserva. Puedes reintentar o cancelar."
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ExitState;

    #[test]
    fn exit_waits_for_decision_and_durable_writes() {
        for state in [ExitState::Running, ExitState::Prompt, ExitState::Closing] {
            assert!(!state.can_close(false, false, false));
        }
        assert!(!ExitState::Saving.can_close(true, false, false));
        assert!(ExitState::Saving.can_close(false, false, false));
        assert!(ExitState::Discarding.can_close(true, false, false));
        for state in [ExitState::Saving, ExitState::Discarding] {
            assert!(!state.can_close(false, true, false));
            assert!(!state.can_close(false, false, true));
        }
    }
}
