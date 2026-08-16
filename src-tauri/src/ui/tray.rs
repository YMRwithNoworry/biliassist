use super::platform;
use anyhow::{anyhow, Context as _};
use gpui::{AnyWindowHandle, AsyncApp};
use std::time::Duration;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
};

const SHOW_MENU_ID: &str = "biliassist.show-main-window";
const QUIT_MENU_ID: &str = "biliassist.quit";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TrayAction {
    Show,
    Quit,
}

pub fn install(main_window: AnyWindowHandle, cx: &mut AsyncApp) -> anyhow::Result<()> {
    let tray_guard = start_tray()?;
    main_window
        .update(cx, |_, window, cx| {
            window.on_window_should_close(cx, |window, cx| {
                platform::hide_main_window(window, cx);
                false
            });
        })
        .context("无法注册窗口关闭事件")?;

    monitor_events(main_window, tray_guard, cx);
    Ok(())
}

fn build_tray() -> anyhow::Result<TrayIcon> {
    let show_item = MenuItem::with_id(SHOW_MENU_ID, "显示主窗口", true, None);
    let quit_item = MenuItem::with_id(QUIT_MENU_ID, "退出程序", true, None);
    let separator = PredefinedMenuItem::separator();
    let menu = Menu::new();
    menu.append(&show_item)?;
    menu.append(&separator)?;
    menu.append(&quit_item)?;

    let icon = load_icon()?;
    TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("BiliAssist")
        .with_icon(icon)
        .with_menu_on_left_click(false)
        .build()
        .map_err(|error| anyhow!("创建系统托盘失败：{error}"))
}

fn load_icon() -> anyhow::Result<Icon> {
    let image = image::load_from_memory(include_bytes!("../../icons/32x32.png"))?.to_rgba8();
    let (width, height) = image.dimensions();
    Icon::from_rgba(image.into_raw(), width, height)
        .map_err(|error| anyhow!("加载系统托盘图标失败：{error}"))
}

#[cfg(not(target_os = "linux"))]
type TrayGuard = TrayIcon;

#[cfg(not(target_os = "linux"))]
fn start_tray() -> anyhow::Result<TrayGuard> {
    build_tray()
}

#[cfg(target_os = "linux")]
struct TrayGuard {
    _thread: std::thread::JoinHandle<()>,
}

#[cfg(target_os = "linux")]
fn start_tray() -> anyhow::Result<TrayGuard> {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let thread = std::thread::Builder::new()
        .name("biliassist-tray".into())
        .spawn(move || {
            let result = (|| -> anyhow::Result<TrayIcon> {
                gtk::init().context("初始化 GTK 托盘事件循环失败")?;
                build_tray()
            })();

            match result {
                Ok(tray) => {
                    let _ = sender.send(Ok(()));
                    gtk::main();
                    drop(tray);
                }
                Err(error) => {
                    let message = error.to_string();
                    let _ = sender.send(Err(message.clone()));
                    log::error!("{message}");
                }
            }
        })?;

    let result = receiver
        .recv_timeout(Duration::from_secs(5))
        .map_err(|error| anyhow!("等待系统托盘启动失败：{error}"))?;
    result.map_err(anyhow::Error::msg)?;
    Ok(TrayGuard { _thread: thread })
}

fn monitor_events<G: 'static>(main_window: AnyWindowHandle, tray_guard: G, cx: &mut AsyncApp) {
    cx.spawn(async move |cx| {
        let _tray_guard = tray_guard;
        loop {
            cx.background_executor()
                .timer(Duration::from_millis(100))
                .await;

            while let Ok(event) = MenuEvent::receiver().try_recv() {
                match menu_action(&event) {
                    Some(TrayAction::Show) => show_main_window(main_window, cx),
                    Some(TrayAction::Quit) => {
                        cx.update(|cx| cx.quit());
                        return;
                    }
                    None => {}
                }
            }

            while let Ok(event) = TrayIconEvent::receiver().try_recv() {
                if should_show_window(&event) {
                    show_main_window(main_window, cx);
                }
            }
        }
    })
    .detach();
}

fn show_main_window(main_window: AnyWindowHandle, cx: &mut AsyncApp) {
    let _ = main_window.update(cx, |_, window, cx| {
        platform::show_main_window(window, cx);
    });
}

fn menu_action(event: &MenuEvent) -> Option<TrayAction> {
    if event.id == SHOW_MENU_ID {
        Some(TrayAction::Show)
    } else if event.id == QUIT_MENU_ID {
        Some(TrayAction::Quit)
    } else {
        None
    }
}

fn should_show_window(event: &TrayIconEvent) -> bool {
    matches!(
        event,
        TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } | TrayIconEvent::DoubleClick {
            button: MouseButton::Left,
            ..
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_tray_menu_actions() {
        assert_eq!(
            menu_action(&MenuEvent {
                id: SHOW_MENU_ID.into(),
            }),
            Some(TrayAction::Show)
        );
        assert_eq!(
            menu_action(&MenuEvent {
                id: QUIT_MENU_ID.into(),
            }),
            Some(TrayAction::Quit)
        );
    }

    #[test]
    fn only_left_click_release_shows_window() {
        let left_release = TrayIconEvent::Click {
            id: "test".into(),
            position: Default::default(),
            rect: Default::default(),
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
        };
        let right_release = TrayIconEvent::Click {
            id: "test".into(),
            position: Default::default(),
            rect: Default::default(),
            button: MouseButton::Right,
            button_state: MouseButtonState::Up,
        };
        assert!(should_show_window(&left_release));
        assert!(!should_show_window(&right_release));
    }
}
