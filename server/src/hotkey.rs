//! Hotkeys are sent through `ydotool`, which injects input at the kernel (uinput)
//! level so compositor binds like Hyprland's `SUPER SHIFT, B` see them.
//! Combos are parsed into Linux keycodes here; no client text reaches a shell.

use std::time::Duration;

use tokio::process::Command;

const MODIFIERS: &[(&str, &[&str], u16)] = &[
    ("super", &["meta", "win", "cmd", "mod", "logo"], 125),
    ("ctrl", &["control", "ctl"], 29),
    ("alt", &["option", "opt"], 56),
    ("altgr", &[], 100),
    ("shift", &[], 42),
];

const KEYS: &[(&str, &[&str], u16)] = &[
    ("escape", &["esc"], 1),
    ("1", &[], 2), ("2", &[], 3), ("3", &[], 4), ("4", &[], 5), ("5", &[], 6),
    ("6", &[], 7), ("7", &[], 8), ("8", &[], 9), ("9", &[], 10), ("0", &[], 11),
    ("minus", &["-"], 12),
    ("equal", &["="], 13),
    ("backspace", &[], 14),
    ("tab", &[], 15),
    ("q", &[], 16), ("w", &[], 17), ("e", &[], 18), ("r", &[], 19), ("t", &[], 20),
    ("y", &[], 21), ("u", &[], 22), ("i", &[], 23), ("o", &[], 24), ("p", &[], 25),
    ("leftbracket", &["["], 26),
    ("rightbracket", &["]"], 27),
    ("enter", &["return"], 28),
    ("a", &[], 30), ("s", &[], 31), ("d", &[], 32), ("f", &[], 33), ("g", &[], 34),
    ("h", &[], 35), ("j", &[], 36), ("k", &[], 37), ("l", &[], 38),
    ("semicolon", &[";"], 39),
    ("apostrophe", &["'"], 40),
    ("grave", &["`"], 41),
    ("backslash", &["\\"], 43),
    ("z", &[], 44), ("x", &[], 45), ("c", &[], 46), ("v", &[], 47), ("b", &[], 48),
    ("n", &[], 49), ("m", &[], 50),
    ("comma", &[","], 51),
    ("period", &["."], 52),
    ("slash", &["/"], 53),
    ("space", &[], 57),
    ("f1", &[], 59), ("f2", &[], 60), ("f3", &[], 61), ("f4", &[], 62), ("f5", &[], 63),
    ("f6", &[], 64), ("f7", &[], 65), ("f8", &[], 66), ("f9", &[], 67), ("f10", &[], 68),
    ("f11", &[], 87), ("f12", &[], 88),
    ("f13", &[], 183), ("f14", &[], 184), ("f15", &[], 185), ("f16", &[], 186),
    ("f17", &[], 187), ("f18", &[], 188), ("f19", &[], 189), ("f20", &[], 190),
    ("f21", &[], 191), ("f22", &[], 192), ("f23", &[], 193), ("f24", &[], 194),
    ("printscreen", &["print", "sysrq"], 99),
    ("home", &[], 102),
    ("up", &[], 103),
    ("pageup", &["pgup"], 104),
    ("left", &[], 105),
    ("right", &[], 106),
    ("end", &[], 107),
    ("down", &[], 108),
    ("pagedown", &["pgdn"], 109),
    ("insert", &["ins"], 110),
    ("delete", &["del"], 111),
    ("mute", &[], 113),
    ("volumedown", &[], 114),
    ("volumeup", &[], 115),
    ("nexttrack", &["next"], 163),
    ("playpause", &["play"], 164),
    ("prevtrack", &["previous", "prev"], 165),
];

const SEND_TIMEOUT: Duration = Duration::from_secs(3);

struct Combo {
    modifiers: Vec<(&'static str, u16)>,
    key: (&'static str, u16),
}

fn lookup(table: &'static [(&'static str, &'static [&'static str], u16)], name: &str) -> Option<(&'static str, u16)> {
    table
        .iter()
        .find(|(canon, aliases, _)| *canon == name || aliases.contains(&name))
        .map(|(canon, _, code)| (*canon, *code))
}

fn parse(combo: &str) -> Result<Combo, String> {
    let mut modifiers: Vec<(&'static str, u16)> = Vec::new();
    let mut key = None;

    for part in combo.split('+').map(|p| p.trim().to_ascii_lowercase()) {
        if part.is_empty() {
            return Err(format!("invalid hotkey \"{combo}\""));
        }
        if let Some(m) = lookup(MODIFIERS, &part) {
            if !modifiers.contains(&m) {
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
    modifiers.sort_by_key(|(name, _)| MODIFIERS.iter().position(|(m, _, _)| m == name));
    Ok(Combo { modifiers, key })
}

/// Validates a combo like `Super + Shift + B` and returns its canonical form `super+shift+b`.
pub fn normalize(combo: &str) -> Result<String, String> {
    let combo = parse(combo)?;
    let mut parts: Vec<&str> = combo.modifiers.iter().map(|(n, _)| *n).collect();
    parts.push(combo.key.0);
    Ok(parts.join("+"))
}

pub async fn send(combo: &str) -> Result<(), String> {
    let combo = parse(combo)?;
    let codes: Vec<u16> = combo.modifiers.iter().map(|(_, c)| *c).chain([combo.key.1]).collect();

    let mut args: Vec<String> = codes.iter().map(|c| format!("{c}:1")).collect();
    args.extend(codes.iter().rev().map(|c| format!("{c}:0")));

    let output = Command::new("ydotool").arg("key").args(&args).kill_on_drop(true).output();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_aliases_and_order() {
        assert_eq!(normalize("Shift + Super + B").unwrap(), "super+shift+b");
        assert_eq!(normalize("meta+shift+b").unwrap(), "super+shift+b");
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
}
