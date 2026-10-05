use crate::{gpui_app::LumenCatApp, worker::Worker};
use gpui::*;

pub fn run(project: Option<String>, settings_directory: Option<std::path::PathBuf>) {
    gpui::application()
        .with_assets(gpui::assets::AllAssets)
        .run(move |cx: &mut App| {
            gpui::init(cx);
            gpui::component::set_locale("es");
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

            gpui::open_window(options, cx, move |window, cx| {
                let settings_path = settings_directory
                    .unwrap_or_else(super::theme::settings_directory)
                    .join("appearance.json");
                let app = cx.new(|cx| LumenCatApp::new(sender, project, settings_path, cx));
                app.read(cx)
                    .target_input
                    .focus_handle
                    .clone()
                    .focus(window, cx);
                let closing = app.downgrade();
                window.on_window_should_close(cx, move |window, cx| {
                    closing
                        .update(cx, |this, cx| {
                            this.request_exit(window, cx);
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
                                cx.update(|window, cx| autosave.update(cx, |this, cx| {
                                    this.save(false);
                                    this.advance_exit();
                                    this.finish_exit(window, cx);
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
                                Ok(crate::worker::Data::Selected(_)
                                    | crate::worker::Data::History(_))
                            );
                            let res = cx.update(|window, cx| {
                                app_clone.update(cx, |this, cx| {
                                    this.on_reply(reply);
                                    if focus_editor && this.exit_state == super::ExitState::Running
                                    {
                                        this.target_input.focus_handle.focus(window, cx);
                                    }
                                    cx.notify();
                                    this.finish_exit(window, cx);
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
