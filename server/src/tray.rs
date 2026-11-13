//! Tray icon for the Windows release build.
//!
//! The icon is created on its own thread, which also pumps the Windows message
//! queue. Open shows the board in the browser. Quit asks the server to stop.
//! A second launch of the same exe opens the board and exits.

use std::{net::SocketAddr, os::windows::ffi::OsStrExt, sync::Mutex};

use tray_icon::{
    Icon, MouseButton, TrayIcon, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuId, MenuItem},
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError},
    System::Threading::CreateMutexW,
    UI::{
        Shell::ShellExecuteW,
        WindowsAndMessaging::{
            DispatchMessageW, GetMessageW, MB_ICONERROR, MB_OK, MSG, MessageBoxW, SW_SHOWNORMAL,
            TranslateMessage,
        },
    },
};

const MUTEX_NAME: &str = "Local\\TheBoard";

/// Address the tray's Open item should load. An unspecified bind becomes localhost.
pub fn board_url(addr: SocketAddr) -> String {
    let host = if addr.ip().is_unspecified() {
        "127.0.0.1".to_string()
    } else if addr.is_ipv6() {
        format!("[{}]", addr.ip())
    } else {
        addr.ip().to_string()
    };
    format!("http://{host}:{}", addr.port())
}

/// Same as [`board_url`] for the `server.bind` string in config.
pub fn board_url_from_bind(bind: &str) -> String {
    bind.parse::<SocketAddr>()
        .map(board_url)
        .unwrap_or_else(|_| "http://127.0.0.1:7878".to_string())
}

/// When another TheBoard process is already running, open `url` and return true.
pub fn focus_existing(url: &str) -> bool {
    if hold_single_instance() {
        false
    } else {
        open_url(url);
        true
    }
}

/// Opens `url` with the default browser.
pub fn open_url(url: &str) {
    let operation = wide("open");
    let file = wide(url);
    let code = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        ) as usize
    };
    if code <= 32 {
        tracing::error!("opening {url} failed (error {code})");
    }
}

/// Shows a dialog. Used when a release build has no console to print an error.
#[cfg_attr(debug_assertions, allow(dead_code))]
pub fn alert(message: &str) {
    let text = wide(message);
    let title = wide("TheBoard");
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}

/// Starts the tray thread. `quit` fires when the user picks Quit.
pub fn spawn(url: String, quit: tokio::sync::oneshot::Sender<()>) {
    let (boot_tx, boot_rx) = std::sync::mpsc::sync_channel(1);
    let started = std::thread::Builder::new()
        .name("theboard-tray".into())
        .spawn(move || {
            let Ok(quit) = boot_rx.recv() else {
                return;
            };
            let _ = run_tray(url, quit);
        });
    if let Err(err) = started {
        tracing::error!("tray thread failed to start: {err}");
        #[cfg(not(debug_assertions))]
        alert(&format!(
            "TheBoard could not start its tray icon, so it will exit.\n\n{err}"
        ));
        #[cfg(debug_assertions)]
        std::mem::forget(quit);
        return;
    }
    let _ = boot_tx.send(quit);
}

/// Drops the quit signal in a release build so a failed tray still stops the server.
/// A debug build keeps the signal alive; Ctrl+C in the terminal still stops it.
struct QuitOnDrop(Option<tokio::sync::oneshot::Sender<()>>);

impl QuitOnDrop {
    fn take(&mut self) -> tokio::sync::oneshot::Sender<()> {
        self.0.take().expect("quit sender")
    }
}

impl Drop for QuitOnDrop {
    fn drop(&mut self) {
        if cfg!(debug_assertions) {
            if let Some(sender) = self.0.take() {
                std::mem::forget(sender);
            }
        }
    }
}

fn run_tray(url: String, quit: tokio::sync::oneshot::Sender<()>) -> Result<(), String> {
    let mut quit_guard = QuitOnDrop(Some(quit));
    let result = pump_tray(&mut quit_guard, url);
    if let Err(err) = &result {
        tracing::error!("tray icon: {err}");
        #[cfg(not(debug_assertions))]
        alert(&format!(
            "TheBoard could not start its tray icon, so it will exit.\n\n{err}"
        ));
    }
    result
}

fn pump_tray(quit_guard: &mut QuitOnDrop, url: String) -> Result<(), String> {
    let menu = Menu::new();
    let open = MenuItem::new("Open", true, None);
    let obs = MenuItem::new("OBS settings", true, None);
    let quit_item = MenuItem::new("Quit", true, None);
    let open_id = open.id().clone();
    let obs_id = obs.id().clone();
    let quit_id = quit_item.id().clone();
    menu.append_items(&[&open, &obs, &quit_item])
        .map_err(|err| err.to_string())?;

    let icon = tray_icon()?;
    let _tray: TrayIcon = tray_icon::TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("TheBoard")
        .with_icon(icon)
        .with_menu_on_left_click(false)
        .with_menu_on_right_click(true)
        .build()
        .map_err(|err| err.to_string())?;
    tracing::info!("tray icon ready");

    let mut quit = Some(quit_guard.take());
    loop {
        let mut message = unsafe { std::mem::zeroed::<MSG>() };
        let status = unsafe { GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) };
        if status <= 0 {
            break;
        }
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        drain_events(&url, &open_id, &obs_id, &quit_id, &mut quit);
    }
    Ok(())
}

fn drain_events(
    url: &str,
    open_id: &MenuId,
    obs_id: &MenuId,
    quit_id: &MenuId,
    quit: &mut Option<tokio::sync::oneshot::Sender<()>>,
) {
    while let Ok(event) = MenuEvent::receiver().try_recv() {
        if event.id == *open_id {
            open_url(url);
        } else if event.id == *obs_id {
            open_url(&format!("{url}#obs"));
        } else if event.id == *quit_id {
            if let Some(sender) = quit.take() {
                let _ = sender.send(());
            }
        }
    }
    while let Ok(event) = TrayIconEvent::receiver().try_recv() {
        if let TrayIconEvent::DoubleClick {
            button: MouseButton::Left,
            ..
        } = event
        {
            open_url(url);
        }
    }
}

fn tray_icon() -> Result<Icon, String> {
    const SIZE: u32 = 32;
    let rgba = include_bytes!("../../assets/theboard-32.rgba");
    Icon::from_rgba(rgba.to_vec(), SIZE, SIZE).map_err(|err| format!("icon: {err:?}"))
}

/// Keeps a named mutex for the life of this process.
/// Returns false when another process already owns it.
fn hold_single_instance() -> bool {
    static HELD: Mutex<Option<isize>> = Mutex::new(None);
    let mut held = HELD.lock().unwrap_or_else(|err| err.into_inner());
    if held.is_some() {
        return true;
    }
    let name = wide(MUTEX_NAME);
    let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
    if handle.is_null() {
        return true;
    }
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        unsafe {
            CloseHandle(handle);
        }
        return false;
    }
    *held = Some(handle as isize);
    true
}

fn wide(text: &str) -> Vec<u16> {
    std::ffi::OsStr::new(text)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::board_url_from_bind;

    #[test]
    fn unspecified_bind_opens_localhost() {
        assert_eq!(board_url_from_bind("0.0.0.0:7878"), "http://127.0.0.1:7878");
        assert_eq!(board_url_from_bind("[::]:9000"), "http://127.0.0.1:9000");
    }

    #[test]
    fn specific_bind_keeps_its_address() {
        assert_eq!(
            board_url_from_bind("192.168.1.101:7878"),
            "http://192.168.1.101:7878"
        );
        assert_eq!(board_url_from_bind("[::1]:7878"), "http://[::1]:7878");
    }
}
