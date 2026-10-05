use super::{
    theme::{Accent, Appearance, ColorMode},
    *,
};
use gpui::assets::IconName;
use gpui::component::{
    Icon,
    button::{Button, ButtonCustomVariant},
    radio::{Radio, RadioGroup},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum FileSection {
    Open,
    Documents,
    New,
}

impl FileSection {
    fn label(self) -> &'static str {
        match self {
            Self::Open => "Abrir",
            Self::Documents => "Documentos del proyecto",
            Self::New => "Nuevo",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum RibbonTab {
    File,
    Home,
    Review,
    Advanced,
    View,
    Addins,
    Settings,
}

impl RibbonTab {
    pub(super) fn is_page(self) -> bool {
        matches!(self, Self::File | Self::Addins | Self::Settings)
    }
    fn label(self) -> &'static str {
        match self {
            Self::File => "Archivo",
            Self::Home => "Inicio",
            Self::Review => "Revisión",
            Self::Advanced => "Avanzado",
            Self::View => "Ver",
            Self::Addins => "Complementos",
            Self::Settings => "Configuración",
        }
    }
    fn id(self) -> &'static str {
        match self {
            Self::File => "ribbon-file",
            Self::Home => "ribbon-home",
            Self::Review => "ribbon-review",
            Self::Advanced => "ribbon-advanced",
            Self::View => "ribbon-view",
            Self::Addins => "ribbon-addins",
            Self::Settings => "ribbon-settings",
        }
    }
}

#[derive(Clone, Copy)]
enum Command {
    Editor,
    NewProject,
    OpenProject,
    ImportDocument,
    Save,
    ExportDocument,
    ImportMemory,
    ExportMemory,
    UpdateSdltm,
    MemoryResources,
    Concordance,
    Replace,
    Verify,
    Qa,
    Terms,
    Tm,
    Confirm,
    Lock,
    Documents,
    Tools,
    ResetPanels,
    Settings,
    Quit,
}

impl Command {
    fn icon(self) -> IconName {
        match self {
            Self::Editor => IconName::ArrowLeft,
            Self::NewProject => IconName::FilePlus,
            Self::OpenProject => IconName::FolderOpen,
            Self::ImportDocument | Self::ImportMemory => IconName::Upload,
            Self::Save | Self::UpdateSdltm => IconName::Save,
            Self::ExportDocument | Self::ExportMemory => IconName::Download,
            Self::MemoryResources | Self::Tm => IconName::BookOpen,
            Self::Concordance | Self::Replace => IconName::Search,
            Self::Verify | Self::Qa => IconName::ShieldCheck,
            Self::Terms => IconName::Languages,
            Self::Confirm => IconName::Check,
            Self::Lock => IconName::Lock,
            Self::Documents => IconName::PanelLeft,
            Self::Tools => IconName::PanelRight,
            Self::ResetPanels => IconName::PanelsTopLeft,
            Self::Settings => IconName::Settings,
            Self::Quit => IconName::LogOut,
        }
    }

    fn label(self, app: &LumenCatApp) -> &'static str {
        match self {
            Self::Editor => "Volver al editor",
            Self::NewProject => "Nuevo proyecto",
            Self::OpenProject => "Abrir proyecto",
            Self::ImportDocument => "Importar documento",
            Self::Save => "Guardar",
            Self::ExportDocument => "Exportar documento",
            Self::ImportMemory => "Importar memoria",
            Self::ExportMemory => "Exportar TMX",
            Self::UpdateSdltm => "Actualizar SDLTM (copia, requiere Trados)",
            Self::MemoryResources => "Memorias del proyecto",
            Self::Concordance => "Concordancia",
            Self::Replace => "Buscar y reemplazar",
            Self::Verify => "Verificar segmento",
            Self::Qa => "Control de calidad",
            Self::Terms => "Terminología",
            Self::Tm => "Memoria de traducción",
            Self::Confirm => "Confirmar sin avanzar",
            Self::Lock => "Bloquear / desbloquear",
            Self::Documents => {
                if app.show_documents {
                    "Ocultar documentos"
                } else {
                    "Mostrar documentos"
                }
            }
            Self::Tools => {
                if app.show_tools {
                    "Ocultar panel de resultados"
                } else {
                    "Mostrar panel de resultados"
                }
            }
            Self::ResetPanels => "Restaurar paneles",
            Self::Settings => "Configuración",
            Self::Quit => "Salir",
        }
    }
    fn id(self) -> &'static str {
        match self {
            Self::Editor => "file-back",
            Self::NewProject => "project-new",
            Self::OpenProject => "project-open",
            Self::ImportDocument => "document-import",
            Self::Save => "project-save",
            Self::ExportDocument => "document-export",
            Self::ImportMemory => "tm-import",
            Self::ExportMemory => "tm-export",
            Self::UpdateSdltm => "tm-update-sdltm",
            Self::MemoryResources => "memory-menu",
            Self::Concordance => "ribbon-concordance",
            Self::Replace => "ribbon-replace",
            Self::Verify => "ribbon-verify",
            Self::Qa => "ribbon-qa",
            Self::Terms => "ribbon-terms",
            Self::Tm => "ribbon-tm",
            Self::Confirm => "ribbon-confirm",
            Self::Lock => "ribbon-lock",
            Self::Documents => "view-documents",
            Self::Tools => "view-tools",
            Self::ResetPanels => "view-reset",
            Self::Settings => "file-settings",
            Self::Quit => "file-quit",
        }
    }
    fn enabled(self, app: &LumenCatApp) -> bool {
        let ready = app.opened && !app.is_busy() && !app.is_dirty();
        match self {
            Self::NewProject | Self::OpenProject => !app.is_busy() && !app.is_dirty(),
            Self::ImportDocument
            | Self::ImportMemory
            | Self::ExportMemory
            | Self::MemoryResources => ready,
            Self::UpdateSdltm => ready && crate::formats::sdltm::available(),
            Self::Save => app.opened && !app.is_busy() && !app.save_error,
            Self::ExportDocument => ready && app.current_document_id.is_some(),
            Self::Concordance | Self::Replace => {
                app.current_document_id.is_some() && !app.is_busy()
            }
            Self::Verify | Self::Lock => app.active_draft.is_some() && !app.is_busy(),
            Self::Confirm => {
                app.active_draft
                    .as_ref()
                    .is_some_and(|d| !d.segment.locked && !d.segment.target.trim().is_empty())
                    && !app.is_busy()
            }
            _ => true,
        }
    }
}

impl LumenCatApp {
    fn run_ribbon_command(
        &mut self,
        command: Command,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !command.enabled(self) {
            return;
        }
        match command {
            Command::Editor => {
                self.ribbon_tab = RibbonTab::Home;
                self.target_input.focus_handle.focus(window, cx);
            }
            Command::NewProject => self.project_dialog(true),
            Command::OpenProject => self.open_project_dialog(),
            Command::ImportDocument => {
                self.import_document_dialog();
                self.ribbon_tab = RibbonTab::Home;
            }
            Command::Save => {
                self.save(true);
                if !self.is_dirty() {
                    self.message = "Proyecto guardado".into();
                }
            }
            Command::ExportDocument => self.export_document_dialog(),
            Command::ImportMemory => self.import_tmx_dialog(),
            Command::ExportMemory => self.export_tmx_dialog(),
            Command::UpdateSdltm => self.update_sdltm_dialog(),
            Command::MemoryResources => {
                self.show_memory_actions = !self.show_memory_actions;
                self.show_documents = true;
            }
            Command::Concordance => {
                self.perform_concordance();
                self.active_tab = RightTab::TranslationMemory;
                self.show_tools = true;
            }
            Command::Replace => {
                self.show_replace = true;
                self.search_input.focus_handle.focus(window, cx);
            }
            Command::Verify => {
                self.request_qa();
                self.active_tab = RightTab::QualityAssurance;
                self.show_tools = true;
            }
            Command::Qa | Command::Terms | Command::Tm => {
                self.show_tools = true;
                self.active_tab = match command {
                    Command::Qa => RightTab::QualityAssurance,
                    Command::Terms => RightTab::Terminology,
                    _ => RightTab::TranslationMemory,
                };
            }
            Command::Confirm => self.confirm_active(),
            Command::Lock => self.toggle_active_lock(),
            Command::Documents => self.show_documents = !self.show_documents,
            Command::Tools => self.show_tools = !self.show_tools,
            Command::ResetPanels => {
                self.show_documents = true;
                self.show_tools = true;
                self.active_tab = RightTab::TranslationMemory;
            }
            Command::Settings => self.ribbon_tab = RibbonTab::Settings,
            Command::Quit => self.request_exit(window, cx),
        }
        cx.notify();
    }

    fn command_button(&self, command: Command, entity: Entity<Self>) -> Button {
        let enabled = command.enabled(self);
        custom_button(
            command.id(),
            command.label(self),
            ButtonVariant::Ghost,
            enabled,
            move |_, window, cx| {
                entity.update(cx, |this, cx| this.run_ribbon_command(command, window, cx));
            },
        )
        .icon(command.icon())
    }

    pub(super) fn render_ribbon(&self, entity: Entity<Self>) -> impl IntoElement {
        let tabs = [
            RibbonTab::File,
            RibbonTab::Home,
            RibbonTab::Review,
            RibbonTab::Advanced,
            RibbonTab::View,
            RibbonTab::Addins,
            RibbonTab::Settings,
        ];
        let commands: &[Command] = match self.ribbon_tab {
            RibbonTab::Review => &[
                Command::Verify,
                Command::Qa,
                Command::Terms,
                Command::Confirm,
                Command::Lock,
            ],
            RibbonTab::Advanced => &[
                Command::MemoryResources,
                Command::ImportMemory,
                Command::ExportMemory,
                Command::UpdateSdltm,
                Command::Concordance,
                Command::Replace,
            ],
            RibbonTab::View => &[
                Command::Documents,
                Command::Tools,
                Command::Tm,
                Command::Qa,
                Command::Terms,
                Command::ResetPanels,
            ],
            _ => &[],
        };
        div()
            .id("ribbon")
            .flex()
            .flex_col()
            .flex_shrink_0()
            .bg(Theme::bg_surface())
            .border_b_1()
            .border_color(Theme::border_subtle())
            .child(
                div()
                    .id("ribbon-tabs")
                    .role(Role::TabList)
                    .aria_label("Pestañas principales")
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .px_2()
                    .gap_1()
                    .children(tabs.into_iter().map(|tab| {
                        let selected = self.ribbon_tab == tab;
                        let click = entity.clone();
                        let keyboard = entity.clone();
                        div()
                            .id(tab.id())
                            .accessibility_id(tab.id())
                            .role(Role::Tab)
                            .aria_label(tab.label())
                            .aria_selected(selected)
                            .focusable()
                            .tab_stop(true)
                            .px_3()
                            .py_2()
                            .text_sm()
                            .cursor_pointer()
                            .border_b_2()
                            .border_color(if selected {
                                Theme::sky()
                            } else {
                                rgba(0x00000000)
                            })
                            .text_color(if selected {
                                Theme::sky()
                            } else {
                                Theme::text_secondary()
                            })
                            .hover(|s| s.bg(Theme::bg_hover()))
                            .focus_visible(|s| s.bg(Theme::bg_hover()))
                            .on_click(move |_, _, cx| {
                                click.update(cx, |this, cx| {
                                    this.ribbon_tab = tab;
                                    cx.notify();
                                });
                            })
                            .on_key_down(move |event, _, cx| {
                                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                    keyboard.update(cx, |this, cx| {
                                        this.ribbon_tab = tab;
                                        cx.notify();
                                    });
                                    cx.stop_propagation();
                                }
                            })
                            .child(tab.label())
                    }))
                    .child(div().flex_1())
                    .child(
                        div()
                            .id("project-name")
                            .role(Role::Label)
                            .aria_label(if self.opened {
                                self.project_path.clone()
                            } else {
                                "Sin proyecto".into()
                            })
                            .max_w(px(200.))
                            .overflow_hidden()
                            .text_ellipsis()
                            .text_xs()
                            .text_color(Theme::text_muted())
                            .child(
                                PathBuf::from(&self.project_path)
                                    .file_name()
                                    .map(|p| p.to_string_lossy().into_owned())
                                    .unwrap_or_else(|| "Sin proyecto".into()),
                            ),
                    )
                    .child(history_button(false, self.opened && !self.is_busy(), {
                        let entity = entity.clone();
                        move |_, _, cx| {
                            entity.update(cx, |this, _| this.history(false));
                        }
                    }))
                    .child(history_button(true, self.opened && !self.is_busy(), {
                        let entity = entity.clone();
                        move |_, _, cx| {
                            entity.update(cx, |this, _| this.history(true));
                        }
                    })),
            )
            .when(self.ribbon_tab == RibbonTab::Home, |d| {
                d.child(self.render_editor_toolbar(entity.clone()))
            })
            .when(!commands.is_empty(), |d| {
                d.child(
                    div()
                        .id("ribbon-commands")
                        .role(Role::Toolbar)
                        .aria_label(format!("Comandos de {}", self.ribbon_tab.label()))
                        .flex()
                        .flex_wrap()
                        .gap_2()
                        .px_3()
                        .py_2()
                        .children(
                            commands
                                .iter()
                                .map(|c| self.command_button(*c, entity.clone())),
                        ),
                )
            })
    }

    pub(super) fn render_ribbon_page(&self, entity: Entity<Self>, cx: &App) -> AnyElement {
        match self.ribbon_tab {
            RibbonTab::File => self.render_file_page(entity, cx).into_any_element(),
            RibbonTab::Settings => self.render_appearance_settings(entity).into_any_element(),
            RibbonTab::Addins => div().id("addins-page").role(Role::TabPanel).aria_label("Complementos").flex_1().p_8().flex().flex_col().gap_4()
                .child(div().text_xl().font_weight(FontWeight::SEMIBOLD).child("Complementos"))
                .child(div().text_sm().child("LumenCAT todavía no dispone de un sistema de plugins. No instala ni carga extensiones de Trados."))
                .child(div().text_sm().text_color(Theme::text_secondary()).child("Memorias, terminología y control de calidad son funciones incorporadas. Se accede a ellas desde Avanzado y Revisión."))
                .child(div().flex().child(self.command_button(Command::Editor, entity))).into_any_element(),
            _ => div().into_any_element(),
        }
    }

    fn file_command(
        &self,
        command: Command,
        label: &'static str,
        shortcut: &'static str,
        entity: Entity<Self>,
        cx: &App,
    ) -> Button {
        self.file_navigation_button(
            command.id(),
            label,
            shortcut,
            false,
            command.enabled(self),
            cx,
        )
        .on_click(move |_, window, cx| {
            entity.update(cx, |this, cx| this.run_ribbon_command(command, window, cx));
        })
    }

    fn file_navigation_button(
        &self,
        id: &'static str,
        label: &'static str,
        shortcut: &'static str,
        selected: bool,
        enabled: bool,
        cx: &App,
    ) -> Button {
        use gpui::component::{Disableable, button::ButtonVariants};
        Button::new(id)
            .accessibility_id(id)
            .accessibility_label(label)
            .with_variant(ButtonVariant::Custom(
                ButtonCustomVariant::new(cx)
                    .color(rgb(if selected { 0x426fae } else { 0x315b99 }).into())
                    .foreground(rgb(0xffffff).into())
                    .hover(rgb(0x426fae).into())
                    .active(rgb(0x254a81).into()),
            ))
            .disabled(!enabled)
            .w_full()
            .h(px(40.))
            .rounded_none()
            .px_5()
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .text_sm()
                    .child(label)
                    .child(div().text_xs().child(shortcut)),
            )
    }

    fn file_action(
        &self,
        command: Command,
        description: &'static str,
        shortcut: &'static str,
        entity: Entity<Self>,
    ) -> Button {
        use gpui::component::{Disableable, button::ButtonVariants};
        Button::new(command.id())
            .accessibility_id(command.id())
            .accessibility_label(command.label(self))
            .ghost()
            .disabled(!command.enabled(self))
            .w_full()
            .h(px(72.))
            .px_4()
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(Icon::new(command.icon()).size_6().text_color(Theme::sky()))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(command.label(self)),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(Theme::text_secondary())
                                    .child(description),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(Theme::text_secondary())
                            .child(shortcut),
                    ),
            )
            .on_click(move |_, window, cx| {
                entity.update(cx, |this, cx| this.run_ribbon_command(command, window, cx));
            })
    }

    fn render_file_page(&self, entity: Entity<Self>, cx: &App) -> Stateful<Div> {
        use gpui::component::{Disableable, button::ButtonVariants};
        let sections = [
            (FileSection::Open, "file-open"),
            (FileSection::Documents, "file-documents"),
            (FileSection::New, "file-new"),
        ];
        let divider = || div().h(px(1.)).mx_4().my_2().bg(rgba(0xffffff30));
        let navigation = div()
            .id("file-navigation")
            .role(Role::Navigation)
            .aria_label("Menú Archivo")
            .w(px(280.))
            .flex_shrink_0()
            .h_full()
            .overflow_y_scroll()
            .bg(rgb(0x315b99))
            .py_4()
            .flex()
            .flex_col()
            .child(
                self.file_command(
                    Command::Editor,
                    "Volver al editor",
                    "Esc",
                    entity.clone(),
                    cx,
                )
                .icon(IconName::CircleArrowLeft)
                .mb_5(),
            )
            .child(self.file_command(Command::Save, "Guardar", "Ctrl+S", entity.clone(), cx))
            .child(self.file_command(
                Command::ExportDocument,
                "Guardar destino como",
                "Mayús+F12",
                entity.clone(),
                cx,
            ))
            .child(divider())
            .children(sections.into_iter().map(|(section, id)| {
                let entity = entity.clone();
                self.file_navigation_button(
                    id,
                    section.label(),
                    "",
                    self.file_section == section,
                    true,
                    cx,
                )
                .on_click(move |_, _, cx| {
                    entity.update(cx, |this, cx| {
                        this.file_section = section;
                        cx.notify();
                    })
                })
            }))
            .child(divider())
            .child(self.file_command(Command::Settings, "Configuración", "", entity.clone(), cx))
            .child(div().flex_1().min_h(px(24.)))
            .child(divider())
            .child(self.file_command(Command::Quit, "Salir", "", entity.clone(), cx));

        let content = div()
            .id("file-content")
            .flex_1()
            .min_w_0()
            .min_h_0()
            .overflow_y_scroll()
            .p_8()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(self.file_section.label()),
            );
        let content = match self.file_section {
            FileSection::Open => content
                .child(self.file_action(Command::OpenProject, "Continúa un proyecto local de LumenCAT.", "Ctrl+O", entity.clone()))
                .child(self.file_action(Command::ImportDocument, "Añade un DOCX, XLIFF 1.2 o TXT al proyecto abierto.", "", entity.clone()))
                .child(self.file_action(Command::ImportMemory, "Importa TMX o SDLTM; la SDLTM se lee en modo de solo lectura (actualizar exige Trados).", "", entity.clone()))
                .child(div().mt_4().max_w(px(650.)).text_sm().text_color(Theme::text_secondary()).child(
                    "No se abren paquetes Trados ni SDLXLIFF. SDLTM se importa desde Avanzado; sin Trados se lee en modo de solo lectura. Para importar documentos o memorias, primero abre o crea un proyecto.",
                )),
            FileSection::New => content
                .child(self.file_action(Command::NewProject, "Elige dónde guardar el nuevo proyecto local.", "Ctrl+N", entity.clone()))
                .child(div().text_sm().text_color(Theme::text_secondary()).child("Después importa los documentos que vas a traducir.")),
            FileSection::Documents => content
                .child(div().text_sm().text_color(Theme::text_secondary()).child(if self.opened { self.project_path.clone() } else { "No hay un proyecto abierto.".into() }))
                .children(self.documents.iter().map(|document| {
                    let entity = entity.clone();
                    let id = document.id;
                    let button_id: SharedString = format!("file-document-{id}").into();
                    Button::new(button_id.clone()).accessibility_id(button_id).accessibility_label(document.name.clone())
                        .ghost().disabled(!self.opened || self.is_busy() || self.is_dirty())
                        .w_full().h(px(48.)).px_4()
                        .child(div().w_full().min_w_0().flex().items_center().gap_4()
                            .child(Icon::new(IconName::FileText).size_6().text_color(Theme::sky()))
                            .child(div().overflow_hidden().text_ellipsis().child(document.name.clone())))
                        .on_click(move |_, window, cx| {
                            entity.update(cx, |this, cx| {
                                if !this.is_busy() && !this.is_dirty() {
                                    this.activate_document(id);
                                    this.ribbon_tab = RibbonTab::Home;
                                    this.target_input.focus_handle.focus(window, cx);
                                    cx.notify();
                                }
                            });
                        })
                }))
                .when(self.documents.is_empty(), |d| d.child(div().text_sm().child("El proyecto no contiene documentos. Impórtalos desde Abrir."))),
        };
        div()
            .id("file-page")
            .accessibility_id("file-page")
            .role(Role::TabPanel)
            .aria_label("Archivo")
            .flex_1()
            .min_h_0()
            .flex()
            .bg(Theme::bg_app())
            .child(navigation)
            .child(content)
    }

    fn change_appearance(&mut self, appearance: Appearance, cx: &mut Context<Self>) {
        match appearance.save(&self.appearance_path) {
            Ok(()) => {
                self.appearance = appearance;
                Theme::apply(appearance);
                Theme::sync_components(cx);
                self.appearance_error = None;
            }
            Err(error) => {
                self.appearance_error = Some(format!(
                    "No se pudo guardar la apariencia: {error}. Se conserva el tema anterior."
                ))
            }
        }
        cx.notify();
    }

    fn appearance_button(
        &self,
        id: &'static str,
        label: &'static str,
        appearance: Appearance,
        selected: bool,
        entity: Entity<Self>,
    ) -> Radio {
        Radio::new(id)
            .accessibility_id(id)
            .label(label)
            .checked(selected)
            .on_change(move |_, _, cx| {
                entity.update(cx, |this, cx| this.change_appearance(appearance, cx));
            })
    }

    fn render_appearance_settings(&self, entity: Entity<Self>) -> Stateful<Div> {
        div().id("settings-page").accessibility_id("settings-page").role(Role::TabPanel).aria_label("Configuración de apariencia").flex_1().min_h_0().overflow_y_scroll().p_8().flex().flex_col().gap_4()
            .child(div().text_xl().font_weight(FontWeight::SEMIBOLD).child("Configuración"))
            .child(div().text_sm().text_color(Theme::text_secondary()).child("La apariencia se aplica al instante y se conserva al volver a abrir LumenCAT. No modifica los documentos ni sus traducciones."))
            .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).mt_4().child("Modo de color"))
            .child(RadioGroup::horizontal("appearance-modes").flex_none().selected_index(Some(if self.appearance.mode == ColorMode::Light { 0 } else { 1 })).children([(ColorMode::Light, "appearance-light", "Modo claro"), (ColorMode::Dark, "appearance-dark", "Modo oscuro")].into_iter().map(|(mode, id, label)| {
                self.appearance_button(id, label, Appearance { mode, ..self.appearance }, self.appearance.mode == mode, entity.clone())
            })))
            .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).mt_4().child("Tema"))
            .child(RadioGroup::horizontal("appearance-accents").flex_none().selected_index(Some(match self.appearance.accent { Accent::Blue => 0, Accent::Teal => 1, Accent::Violet => 2 })).children([(Accent::Blue, "theme-blue", "Azul"), (Accent::Teal, "theme-teal", "Verde petróleo"), (Accent::Violet, "theme-violet", "Violeta")].into_iter().map(|(accent, id, label)| {
                self.appearance_button(id, label, Appearance { accent, ..self.appearance }, self.appearance.accent == accent, entity.clone())
            })))
            .when_some(self.appearance_error.as_ref(), |d, error| d.child(div().id("appearance-error").role(Role::Alert).text_color(Theme::rose()).child(error.clone())))
            .child(div().text_xs().text_color(Theme::text_muted()).child(format!("Preferencia local: {}", self.appearance_path.display())))
            .child(div().mt_4().flex().child(self.command_button(Command::Editor, entity)))
    }
}
