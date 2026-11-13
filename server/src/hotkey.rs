//! Hotkeys are parsed from combos like `super+shift+b` into per-platform key codes,
//! so no client text ever reaches a shell or the OS as-is.
//!
//! - Linux: `ydotool`, which injects at the uinput level so compositor binds
//!   (e.g. Hyprland's `SUPER SHIFT, B`) see the keys.
//! - Windows: `SendInput`. Windows won't deliver synthetic input to windows running
//!   as administrator unless the server is elevated too, and never for Ctrl+Alt+Del.

struct KeyDef {
    name: &'static str,
    aliases: &'static [&'static str],
    /// Linux input event code (`KEY_*`).
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    linux: u16,
    /// Windows virtual-key code (`VK_*`).
    #[cfg_attr(not(windows), allow(dead_code))]
    vk: u16,
    /// Needs `KEYEVENTF_EXTENDEDKEY` on Windows.
    #[cfg_attr(not(windows), allow(dead_code))]
    extended: bool,
}

const fn key(name: &'static str, aliases: &'static [&'static str], linux: u16, vk: u16) -> KeyDef {
    KeyDef {
        name,
        aliases,
        linux,
        vk,
        extended: false,
    }
}

const fn ext(name: &'static str, aliases: &'static [&'static str], linux: u16, vk: u16) -> KeyDef {
    KeyDef {
        name,
        aliases,
        linux,
        vk,
        extended: true,
    }
}

/// Listed in the order they're pressed and displayed.
const MODIFIERS: &[KeyDef] = &[
    ext(
        "super",
        &["meta", "win", "windows", "cmd", "mod", "logo"],
        125,
        0x5B,
    ),
    key("ctrl", &["control", "ctl"], 29, 0xA2),
    key("alt", &["option", "opt"], 56, 0xA4),
    ext("altgr", &[], 100, 0xA5),
    key("shift", &[], 42, 0xA0),
];

const KEYS: &[KeyDef] = &[
    key("escape", &["esc"], 1, 0x1B),
    key("1", &[], 2, 0x31),
    key("2", &[], 3, 0x32),
    key("3", &[], 4, 0x33),
    key("4", &[], 5, 0x34),
    key("5", &[], 6, 0x35),
    key("6", &[], 7, 0x36),
    key("7", &[], 8, 0x37),
    key("8", &[], 9, 0x38),
    key("9", &[], 10, 0x39),
    key("0", &[], 11, 0x30),
    key("minus", &["-"], 12, 0xBD),
    key("equal", &["="], 13, 0xBB),
    key("backspace", &[], 14, 0x08),
    key("tab", &[], 15, 0x09),
    key("q", &[], 16, 0x51),
    key("w", &[], 17, 0x57),
    key("e", &[], 18, 0x45),
    key("r", &[], 19, 0x52),
    key("t", &[], 20, 0x54),
    key("y", &[], 21, 0x59),
    key("u", &[], 22, 0x55),
    key("i", &[], 23, 0x49),
    key("o", &[], 24, 0x4F),
    key("p", &[], 25, 0x50),
    key("leftbracket", &["["], 26, 0xDB),
    key("rightbracket", &["]"], 27, 0xDD),
    key("enter", &["return"], 28, 0x0D),
    key("a", &[], 30, 0x41),
    key("s", &[], 31, 0x53),
    key("d", &[], 32, 0x44),
    key("f", &[], 33, 0x46),
    key("g", &[], 34, 0x47),
    key("h", &[], 35, 0x48),
    key("j", &[], 36, 0x4A),
    key("k", &[], 37, 0x4B),
    key("l", &[], 38, 0x4C),
    key("semicolon", &[";"], 39, 0xBA),
    key("apostrophe", &["'"], 40, 0xDE),
    key("grave", &["`"], 41, 0xC0),
    key("backslash", &["\\"], 43, 0xDC),
    key("z", &[], 44, 0x5A),
    key("x", &[], 45, 0x58),
    key("c", &[], 46, 0x43),
    key("v", &[], 47, 0x56),
    key("b", &[], 48, 0x42),
    key("n", &[], 49, 0x4E),
    key("m", &[], 50, 0x4D),
    key("comma", &[","], 51, 0xBC),
    key("period", &["."], 52, 0xBE),
    key("slash", &["/"], 53, 0xBF),
    key("space", &[], 57, 0x20),
    key("f1", &[], 59, 0x70),
    key("f2", &[], 60, 0x71),
    key("f3", &[], 61, 0x72),
    key("f4", &[], 62, 0x73),
    key("f5", &[], 63, 0x74),
    key("f6", &[], 64, 0x75),
    key("f7", &[], 65, 0x76),
    key("f8", &[], 66, 0x77),
    key("f9", &[], 67, 0x78),
    key("f10", &[], 68, 0x79),
    key("f11", &[], 87, 0x7A),
    key("f12", &[], 88, 0x7B),
    key("f13", &[], 183, 0x7C),
    key("f14", &[], 184, 0x7D),
    key("f15", &[], 185, 0x7E),
    key("f16", &[], 186, 0x7F),
    key("f17", &[], 187, 0x80),
    key("f18", &[], 188, 0x81),
    key("f19", &[], 189, 0x82),
    key("f20", &[], 190, 0x83),
    key("f21", &[], 191, 0x84),
    key("f22", &[], 192, 0x85),
    key("f23", &[], 193, 0x86),
    key("f24", &[], 194, 0x87),
    ext("printscreen", &["print", "sysrq"], 99, 0x2C),
    ext("home", &[], 102, 0x24),
    ext("up", &[], 103, 0x26),
    ext("pageup", &["pgup"], 104, 0x21),
    ext("left", &[], 105, 0x25),
    ext("right", &[], 106, 0x27),
    ext("end", &[], 107, 0x23),
    ext("down", &[], 108, 0x28),
    ext("pagedown", &["pgdn"], 109, 0x22),
    ext("insert", &["ins"], 110, 0x2D),
    ext("delete", &["del"], 111, 0x2E),
    ext("mute", &[], 113, 0xAD),
    ext("volumedown", &[], 114, 0xAE),
    ext("volumeup", &[], 115, 0xAF),
    ext("nexttrack", &["next"], 163, 0xB0),
    ext("playpause", &["play"], 164, 0xB3),
    ext("prevtrack", &["previous", "prev"], 165, 0xB1),
];

struct Combo {
    modifiers: Vec<&'static KeyDef>,
    key: &'static KeyDef,
}

impl Combo {
    /// Modifiers then the key, in press order.
    fn keys(&self) -> impl DoubleEndedIterator<Item = &'static KeyDef> + '_ {
        self.modifiers.iter().copied().chain([self.key])
    }
}

fn lookup(table: &'static [KeyDef], name: &str) -> Option<&'static KeyDef> {
    table
        .iter()
        .find(|k| k.name == name || k.aliases.contains(&name))
}

fn parse(combo: &str) -> Result<Combo, String> {
    let mut modifiers: Vec<&'static KeyDef> = Vec::new();
    let mut key = None;

    for part in combo.split('+').map(|p| p.trim().to_ascii_lowercase()) {
        if part.is_empty() {
            return Err(format!("invalid hotkey \"{combo}\""));
        }
        if let Some(m) = lookup(MODIFIERS, &part) {
            if !modifiers.iter().any(|x| x.name == m.name) {
                modifiers.push(m);
            }
        } else if let Some(k) = lookup(KEYS, &part) {
            if key.replace(k).is_some() {
                return Err("a hotkey can only have one non-modifier key".into());
            }
        } else {
            return Err(format!("unknown key \"{part}\""));
        }
    }

    let key = key.ok_or("a hotkey needs a key besides modifiers")?;
    modifiers.sort_by_key(|m| MODIFIERS.iter().position(|x| x.name == m.name));
    Ok(Combo { modifiers, key })
}

/// Validates a combo like `Super + Shift + B` and returns its canonical form `super+shift+b`.
pub fn normalize(combo: &str) -> Result<String, String> {
    let combo = parse(combo)?;
    Ok(combo.keys().map(|k| k.name).collect::<Vec<_>>().join("+"))
}

pub async fn send(combo: &str) -> Result<(), String> {
    platform::send(parse(combo)?).await
}

#[cfg(target_os = "linux")]
mod platform {
    use std::time::Duration;

    use tokio::process::Command;

    use super::Combo;

    const SEND_TIMEOUT: Duration = Duration::from_secs(3);

    pub async fn send(combo: Combo) -> Result<(), String> {
        let mut args: Vec<String> = combo.keys().map(|k| format!("{}:1", k.linux)).collect();
        args.extend(combo.keys().rev().map(|k| format!("{}:0", k.linux)));

        let output = Command::new("ydotool")
            .arg("key")
            .args(&args)
            .kill_on_drop(true)
            .output();
        let output = tokio::time::timeout(SEND_TIMEOUT, output)
            .await
            .map_err(|_| "ydotool timed out (is ydotoold running?)".to_string())?
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => "ydotool is not installed".to_string(),
                _ => format!("running ydotool: {e}"),
            })?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("ydotool failed: {}", stderr.trim()))
        }
    }
}

#[cfg(windows)]
mod platform {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
        SendInput,
    };

    use super::{Combo, KeyDef};

    fn input(key: &KeyDef, up: bool) -> INPUT {
        let mut flags = if key.extended {
            KEYEVENTF_EXTENDEDKEY
        } else {
            0
        };
        if up {
            flags |= KEYEVENTF_KEYUP;
        }
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: key.vk,
                    wScan: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    pub async fn send(combo: Combo) -> Result<(), String> {
        let mut inputs: Vec<INPUT> = combo.keys().map(|k| input(k, false)).collect();
        inputs.extend(combo.keys().rev().map(|k| input(k, true)));

        let sent = tokio::task::spawn_blocking(move || {
            // SAFETY: `inputs` is a valid, initialized slice of INPUT for the duration of the call.
            unsafe {
                SendInput(
                    inputs.len() as u32,
                    inputs.as_ptr(),
                    size_of::<INPUT>() as i32,
                ) as usize
            }
        });
        let expected = combo.keys().count() * 2;
        match sent.await {
            Ok(n) if n == expected => Ok(()),
            Ok(_) => Err("Windows blocked the keypress (is another app capturing input?)".into()),
            Err(e) => Err(format!("sending keys: {e}")),
        }
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
mod platform {
    use super::Combo;

    pub async fn send(_combo: Combo) -> Result<(), String> {
        Err(format!(
            "hotkeys aren't supported on {} yet",
            std::env::consts::OS
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_aliases_and_order() {
        assert_eq!(normalize("Shift + Super + B").unwrap(), "super+shift+b");
        assert_eq!(normalize("win+shift+b").unwrap(), "super+shift+b");
        assert_eq!(normalize("CTRL+alt+Del").unwrap(), "ctrl+alt+delete");
        assert_eq!(normalize("f13").unwrap(), "f13");
    }

    #[test]
    fn rejects_bad_combos() {
        assert!(normalize("super+shift").is_err());
        assert!(normalize("a+b").is_err());
        assert!(normalize("super+banana").is_err());
        assert!(normalize("super++b").is_err());
    }

    #[test]
    fn every_key_has_codes_for_both_platforms() {
        for k in MODIFIERS.iter().chain(KEYS) {
            assert!(k.linux > 0 && k.vk > 0, "{} is missing a code", k.name);
        }
    }
}
