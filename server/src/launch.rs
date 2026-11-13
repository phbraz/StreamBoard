//! Starts an application on the machine running the server.
//!
//! The target is an existing program file, opened directly. Nothing is passed
//! through a shell, and shells and scripts are refused.

use std::path::{Path, PathBuf};

const MAX_PATH_CHARS: usize = 1024;

const BLOCKED_STEMS: &[&str] = &[
    "cmd",
    "powershell",
    "pwsh",
    "bash",
    "sh",
    "wscript",
    "cscript",
    "mshta",
    "conhost",
];

const BLOCKED_EXTENSIONS: &[&str] = &[
    "bat", "cmd", "ps1", "vbs", "js", "jse", "wsf", "scr", "com", "msi", "sh", "bash", "py",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedApp {
    pub name: String,
    pub path: String,
}

/// Checks `raw` is an application this server is willing to start, and returns a normalized path.
pub fn normalize_target(raw: &str) -> Result<String, String> {
    let raw = raw.trim().trim_matches('"').trim();
    if raw.is_empty() {
        return Err("app path is empty".into());
    }
    if raw.chars().count() > MAX_PATH_CHARS {
        return Err("app path is too long".into());
    }
    if raw.chars().any(char::is_control) {
        return Err("app path has invalid characters".into());
    }

    let path = Path::new(raw);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if BLOCKED_STEMS.contains(&stem.as_str()) {
        return Err(format!("\"{stem}\" is a shell, not an app"));
    }
    let ext = extension(path);
    if BLOCKED_EXTENSIONS.contains(&ext.as_str()) {
        return Err(format!(".{ext} files can't be launched"));
    }
    if !allowed_extension(&ext) {
        return Err(allowed_hint());
    }
    if !path.is_file() {
        return Err(format!("app not found: {raw}"));
    }
    #[cfg(target_os = "linux")]
    if ext != "desktop" && !is_executable(path) {
        return Err(format!("{} is not executable", path.display()));
    }

    Ok(display_path(
        &path.canonicalize().unwrap_or_else(|_| path.to_path_buf()),
    ))
}

#[cfg(target_os = "linux")]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .is_ok_and(|meta| meta.permissions().mode() & 0o111 != 0)
}

pub fn label_from_path(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .map(|s| s.to_string_lossy().chars().take(40).collect())
        .filter(|s: &String| !s.is_empty())
        .unwrap_or_else(|| "App".into())
}

pub async fn list() -> Vec<ListedApp> {
    tokio::task::spawn_blocking(list_installed)
        .await
        .unwrap_or_default()
}

pub async fn open(path: &str) -> Result<(), String> {
    let path = normalize_target(path)?;
    tokio::task::spawn_blocking(move || platform::open(Path::new(&path)))
        .await
        .map_err(|e| format!("launching app: {e}"))?
}

fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn display_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    let text = text
        .strip_prefix(r"\\?\UNC\")
        .map(|rest| format!(r"\\{rest}"))
        .unwrap_or_else(|| text.strip_prefix(r"\\?\").unwrap_or(&text).to_string());
    text
}

#[cfg(windows)]
fn allowed_extension(ext: &str) -> bool {
    ext == "exe" || ext == "lnk"
}

#[cfg(windows)]
fn allowed_hint() -> String {
    "use an .exe or a Start menu shortcut (.lnk)".into()
}

#[cfg(target_os = "linux")]
fn allowed_extension(ext: &str) -> bool {
    ext == "desktop" || ext.is_empty() || ext == "appimage"
}

#[cfg(target_os = "linux")]
fn allowed_hint() -> String {
    "use an executable or a .desktop file".into()
}

#[cfg(not(any(windows, target_os = "linux")))]
fn allowed_extension(_ext: &str) -> bool {
    false
}

#[cfg(not(any(windows, target_os = "linux")))]
fn allowed_hint() -> String {
    format!(
        "launching apps isn't supported on {} yet",
        std::env::consts::OS
    )
}

fn list_installed() -> Vec<ListedApp> {
    let mut apps = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for root in app_roots() {
        collect(&root, &root, &mut apps, &mut seen);
    }
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}

fn collect(
    root: &Path,
    dir: &Path,
    out: &mut Vec<ListedApp>,
    seen: &mut std::collections::HashSet<String>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(root, &path, out, seen);
            continue;
        }
        let Some(name) = app_name(root, &path) else {
            continue;
        };
        let Ok(path) = normalize_target(&path.to_string_lossy()) else {
            continue;
        };
        if !seen.insert(path.clone()) {
            continue;
        }
        out.push(ListedApp { name, path });
    }
}

#[cfg(windows)]
fn app_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(data) = dirs::data_dir() {
        roots.push(data.join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    if let Some(program_data) = std::env::var_os("ProgramData") {
        roots.push(PathBuf::from(program_data).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    roots
}

#[cfg(windows)]
fn app_name(root: &Path, path: &Path) -> Option<String> {
    if extension(path) != "lnk" {
        return None;
    }
    let relative = path.strip_prefix(root).unwrap_or(path);
    let name = relative.with_extension("");
    let name = name.to_string_lossy().replace('\\', " / ");
    Some(name)
}

#[cfg(target_os = "linux")]
fn app_roots() -> Vec<PathBuf> {
    let mut roots = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
    ];
    if let Some(data) = dirs::data_dir() {
        roots.push(data.join("applications"));
    }
    roots
}

#[cfg(target_os = "linux")]
fn app_name(_root: &Path, path: &Path) -> Option<String> {
    if extension(path) != "desktop" {
        return None;
    }
    let text = std::fs::read_to_string(path).ok()?;
    let mut name = None;
    let mut application = false;
    for line in text.lines().take(200) {
        let line = line.trim();
        if line.starts_with('[') && line != "[Desktop Entry]" && name.is_some() {
            break;
        }
        if line == "NoDisplay=true" || line == "Hidden=true" {
            return None;
        }
        if line == "Type=Application" {
            application = true;
        }
        if let Some(value) = line.strip_prefix("Name=") {
            if name.is_none() && !value.is_empty() {
                name = Some(value.to_string());
            }
        }
    }
    application.then_some(name).flatten()
}

#[cfg(not(any(windows, target_os = "linux")))]
fn app_roots() -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(not(any(windows, target_os = "linux")))]
fn app_name(_root: &Path, _path: &Path) -> Option<String> {
    None
}

#[cfg(windows)]
mod platform {
    use std::{os::windows::ffi::OsStrExt, path::Path};

    use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};

    fn wide(text: &std::ffi::OsStr) -> Vec<u16> {
        text.encode_wide().chain(std::iter::once(0)).collect()
    }

    pub fn open(path: &Path) -> Result<(), String> {
        let file = wide(path.as_os_str());
        let operation = wide(std::ffi::OsStr::new("open"));
        let directory = path.parent().map(|dir| wide(dir.as_os_str()));
        let code = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                operation.as_ptr(),
                file.as_ptr(),
                std::ptr::null(),
                directory
                    .as_ref()
                    .map_or(std::ptr::null(), |dir| dir.as_ptr()),
                SW_SHOWNORMAL,
            ) as usize
        };
        if code > 32 {
            Ok(())
        } else {
            Err(format!(
                "Windows couldn't start {} (error {code})",
                path.display()
            ))
        }
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use std::{path::Path, process::Stdio};

    pub fn open(path: &Path) -> Result<(), String> {
        let mut command = if super::extension(path) == "desktop" {
            let mut command = std::process::Command::new("gio");
            command.arg("launch").arg(path);
            command
        } else {
            std::process::Command::new(path)
        };
        match command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(mut child) => {
                std::thread::spawn(move || {
                    let _ = child.wait();
                });
                Ok(())
            }
            Err(e)
                if e.kind() == std::io::ErrorKind::NotFound
                    && super::extension(path) == "desktop" =>
            {
                Err("gio is not installed, so .desktop files can't be launched".into())
            }
            Err(e) => Err(format!("starting {}: {e}", path.display())),
        }
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
mod platform {
    use std::path::Path;

    pub fn open(path: &Path) -> Result<(), String> {
        Err(format!(
            "can't start {} on {} yet",
            path.display(),
            std::env::consts::OS
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_shells_scripts_and_blank_paths() {
        assert!(normalize_target("").is_err());
        assert!(normalize_target("   ").is_err());
        assert!(normalize_target("cmd.exe").unwrap_err().contains("shell"));
        assert!(normalize_target("powershell.exe").is_err());
        assert!(
            normalize_target("run.bat")
                .unwrap_err()
                .contains("can't be launched")
        );
        assert!(normalize_target("notes\n.exe").is_err());
    }

    #[cfg(windows)]
    #[test]
    fn accepts_a_real_exe() {
        let dir = std::env::temp_dir().join(format!("soundboard-app-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("widget.exe");
        std::fs::write(&exe, b"").unwrap();
        let got = normalize_target(&exe.to_string_lossy()).unwrap();
        assert!(got.to_ascii_lowercase().ends_with("widget.exe"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
