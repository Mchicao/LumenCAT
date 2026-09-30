use crate::{gpui_app::LumenCatApp, worker::Worker};
use gpui::*;

pub fn run(project: Option<String>) {
    Application::new().run(move |cx: &mut App| {
        let (sender, receiver) = Worker::start_async();

        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(1280.), px(820.)),
                cx,
            ))),
            titlebar: Some(TitlebarOptions {
                title: Some("LumenCAT — Computer-Assisted Translation Studio".into()),
                ..Default::default()
            }),
            ..Default::default()
        };

        cx.open_window(options, move |window, cx| {
            let app = cx.new(|cx| LumenCatApp::new(sender, project, cx));
            app.read(cx).target_input.focus_handle.focus(window);
            let closing = app.downgrade();
            window.on_window_should_close(cx, move |_, cx| {
                closing
                    .update(cx, |this, cx| {
                        if !this.opened {
                            return true;
                        }
                        this.closing_requested = true;
                        this.save(true);
                        this.close_when_saved();
                        this.message = "Guardando antes de cerrar…".into();
                        cx.notify();
                        false
                    })
                    .unwrap_or(true)
            });
            let autosave = app.downgrade();
            let (ticks, tick_receiver) = async_channel::bounded(1);
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(250));
                    if ticks.send_blocking(()).is_err() {
                        break;
                    }
                }
            });
            window
                .spawn(cx, async move |cx| {
                    while tick_receiver.recv().await.is_ok() {
                        if !matches!(
                            cx.update(|_, cx| autosave.update(cx, |this, cx| {
                                this.save(false);
                                this.close_when_saved();
                                cx.notify();
                            })),
                            Ok(Ok(()))
                        ) {
                            break;
                        }
                    }
                })
                .detach();
            let app_clone = app.clone();

            window
                .spawn(cx, async move |cx| {
                    while let Ok(reply) = receiver.recv().await {
                        let focus_editor = matches!(
                            &reply.result,
                            Ok(crate::worker::Data::Selected(_) | crate::worker::Data::History(_))
                        );
                        let res = cx.update(|window, cx| {
                            app_clone.update(cx, |this, cx| {
                                this.on_reply(reply);
                                if focus_editor {
                                    this.target_input.focus_handle.focus(window);
                                }
                                cx.notify();
                                if this.closing_requested && !this.opened {
                                    window.remove_window();
                                    cx.quit();
                                }
                            });
                        });
                        if res.is_err() {
                            break;
                        }
                    }
                })
                .detach();

            app
        })
        .unwrap_or_else(|error| panic!("No se pudo abrir la ventana GPUI: {error}"));
    });
}
