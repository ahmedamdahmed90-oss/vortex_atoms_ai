//! Optional Windows system-tray controller for the API server.
//!
//! Compiled into the binary only with `--features tray`. When the feature is
//! off, [`spawn`] is a compile-time no-op that always returns `None`, so the
//! server code can call it unconditionally.

/// Keeps the tray thread alive for the lifetime of the process (dropping the
/// handle lets the thread keep running; the OS thread is independent).
pub struct TrayGuard(#[allow(dead_code)] std::thread::JoinHandle<()>);

/// Spawn a tray icon (32x32 atom glyph, generated locally) with a context menu:
/// - left click / "Open Dashboard": open `url` in the default browser
/// - "Stop server": send `true` on the `quit` watch channel (graceful shutdown)
/// - "Exit": hard-exit the process
///
/// Returns `None` when built without the `tray` feature or when the icon could
/// not be created.
pub fn spawn(url: String, quit: tokio::sync::watch::Sender<bool>) -> Option<TrayGuard> {
    #[cfg(feature = "tray")]
    {
        imp::spawn_impl(url, quit).map(TrayGuard)
    }
    #[cfg(not(feature = "tray"))]
    {
        let _ = (url, quit);
        None
    }
}

/// Flash a tray notification when a second instance is detected.
/// A no-op when built without the `tray` feature.
pub fn tray_flash(_title: &str, _message: &str) -> crate::Result<()> {
    Ok(())
}

#[cfg(feature = "tray")]
mod imp {
    use std::thread::JoinHandle;

    use tray_icon::menu::{Menu, MenuEvent, MenuItem};
    use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    pub fn spawn_impl(
        url: String,
        quit: tokio::sync::watch::Sender<bool>,
    ) -> Option<JoinHandle<()>> {
        std::thread::Builder::new()
            .name("vortex-tray".to_string())
            .spawn(move || {
                let _ = run_loop(&url, &quit);
            })
            .ok()
    }

    fn run_loop(url: &str, quit: &tokio::sync::watch::Sender<bool>) -> Option<()> {
        let menu = Menu::new();
        let open = MenuItem::new("Open Dashboard", true, None);
        let stop = MenuItem::new("Stop server", true, None);
        let exit = MenuItem::new("Exit", true, None);
        menu.append(&open).ok()?;
        menu.append(&stop).ok()?;
        menu.append(&exit).ok()?;

        let icon = Icon::from_rgba(atom_icon(), 32, 32).ok()?;
        let _tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Vortex Atoms AI - local LLM server")
            .with_icon(icon)
            .build()
            .ok()?;

        let open_id = open.id().clone();
        let stop_id = stop.id().clone();
        let exit_id = exit.id().clone();
        drop(open);
        drop(stop);
        drop(exit);

        pump_and_handle(url, quit, &open_id, &stop_id, &exit_id);
        Some(())
    }

    /// Drain event channels and react to click / menu events.
    fn drain_and_handle(
        url: &str,
        quit: &tokio::sync::watch::Sender<bool>,
        open_id: &tray_icon::menu::MenuId,
        stop_id: &tray_icon::menu::MenuId,
        exit_id: &tray_icon::menu::MenuId,
    ) {
        if let Ok(event) = TrayIconEvent::receiver().try_recv() {
            // Forensic log: every tray interaction is timestamped so an
            // unexpected stop/exit can be traced to a real user gesture.
            match &event {
                TrayIconEvent::Click {
                    button,
                    button_state,
                    ..
                } => {
                    println!("[Tray] click button={button:?} state={button_state:?}");
                }
                TrayIconEvent::DoubleClick { button, .. } => {
                    println!("[Tray] double-click button={button:?}");
                }
                other => {
                    println!("[Tray] event: {other:?}");
                }
            }
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                open_browser(url);
            }
        }
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id().0 == open_id.0 {
                println!("[Tray] menu: Open Dashboard");
                open_browser(url);
            } else if event.id().0 == stop_id.0 {
                println!("[Tray] menu: Stop server — requesting graceful shutdown");
                let _ = quit.send(true);
            } else if event.id().0 == exit_id.0 {
                println!("[Tray] menu: Exit — terminating now");
                std::process::exit(0);
            } else {
                println!("[Tray] menu: unknown item id={}", event.id().0);
            }
        }
    }

    /// On Windows the tray-icon window receives clicks as Win32 messages
    /// (`WM_USER_TRAYICON`) that are only dispatched when the owning thread
    /// pumps the queue. tray-icon does NOT run its own pump, so we must.
    #[cfg(target_os = "windows")]
    fn pump_and_handle(
        url: &str,
        quit: &tokio::sync::watch::Sender<bool>,
        open_id: &tray_icon::menu::MenuId,
        stop_id: &tray_icon::menu::MenuId,
        exit_id: &tray_icon::menu::MenuId,
    ) {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            DispatchMessageW, GetMessageW, TranslateMessage, MSG,
        };
        loop {
            let mut msg: MSG = unsafe { std::mem::zeroed() };
            let ret = unsafe { GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) };
            if ret <= 0 {
                break;
            }
            unsafe {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            drain_and_handle(url, quit, open_id, stop_id, exit_id);
        }
    }

    /// Non-Windows fallback: poll the channels periodically.
    #[cfg(not(target_os = "windows"))]
    fn pump_and_handle(
        url: &str,
        quit: &tokio::sync::watch::Sender<bool>,
        open_id: &tray_icon::menu::MenuId,
        stop_id: &tray_icon::menu::MenuId,
        exit_id: &tray_icon::menu::MenuId,
    ) {
        loop {
            drain_and_handle(url, quit, open_id, stop_id, exit_id);
            std::thread::sleep(std::time::Duration::from_millis(60));
        }
    }

    fn open_browser(url: &str) {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn();
    }

    /// 32x32 RGBA atom glyph: two concentric rings + amber nucleus.
    fn atom_icon() -> Vec<u8> {
        const SIZE: usize = 32;
        const CX: f32 = 15.5;
        const CY: f32 = 15.5;
        let mut px = vec![0u8; SIZE * SIZE * 4];
        for y in 0..SIZE {
            for x in 0..SIZE {
                let dx = x as f32 - CX;
                let dy = y as f32 - CY;
                let d = dx.hypot(dy);
                let (mut r, mut g, mut b) = (0u8, 0u8, 0u8);
                if d <= 2.6 {
                    r = 245;
                    g = 158;
                    b = 11;
                } else if (d - 7.0).abs() <= 1.0 {
                    r = 167;
                    g = 139;
                    b = 250;
                } else if (d - 11.0).abs() <= 1.0 {
                    r = 34;
                    g = 211;
                    b = 238;
                }
                let i = (y * SIZE + x) * 4;
                if r != 0 || g != 0 || b != 0 {
                    px[i] = r;
                    px[i + 1] = g;
                    px[i + 2] = b;
                    px[i + 3] = 255;
                }
            }
        }
        px
    }
}
