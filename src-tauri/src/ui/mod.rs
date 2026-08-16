mod app;
mod auth;
mod cloud;
mod platform;
mod tray;

use crate::{auto_reply, runtime, storage};
use app::{AppView, Bootstrap};
use gpui::{px, size, AppContext, Bounds, WindowBounds, WindowOptions};
use gpui_component::{Root, Theme, ThemeMode};
use gpui_component_assets::Assets;

pub fn run() {
    let bootstrap = runtime().block_on(async {
        let auth = auth::restore_session().await;
        let licensed =
            platform::is_licensed() || auth.as_ref().is_some_and(|session| session.tier == "plus");
        Bootstrap {
            accounts: storage::get_accounts().await.unwrap_or_default(),
            settings: auto_reply::get_settings().await.unwrap_or_default(),
            licensed,
            autostart: platform::is_autostart_enabled().unwrap_or(false),
            auth,
        }
    });

    gpui_platform::application()
        .with_assets(Assets)
        .run(move |cx| {
            gpui_component::init(cx);
            Theme::change(ThemeMode::Dark, None, cx);

            let bounds = Bounds::centered(None, size(px(1180.), px(780.)), cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(900.), px(620.))),
                ..Default::default()
            };
            let bootstrap = bootstrap.clone();

            cx.spawn(async move |cx| {
                let window = cx
                    .open_window(options, |window, cx| {
                        let view = cx.new(|cx| AppView::new(bootstrap, window, cx));
                        cx.new(|cx| Root::new(view, window, cx))
                    })
                    .expect("无法创建 GPUI 主窗口");

                window.update(cx, |_, window, _| {
                    window.set_window_title("B站账号管理工具");
                    window.activate_window();
                })?;

                let main_window = window.into();
                if let Err(error) = tray::install(main_window, cx) {
                    log::error!("无法初始化系统托盘，关闭窗口将退出程序：{error}");
                }
                Ok::<_, anyhow::Error>(())
            })
            .detach();
        });
}
