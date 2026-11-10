// Key names must match the server's table in server/src/hotkey.rs.
const CODE_NAMES: Record<string, string> = {
  Space: 'space',
  Enter: 'enter',
  NumpadEnter: 'enter',
  Tab: 'tab',
  Escape: 'escape',
  Backspace: 'backspace',
  Minus: 'minus',
  Equal: 'equal',
  BracketLeft: 'leftbracket',
  BracketRight: 'rightbracket',
  Semicolon: 'semicolon',
  Quote: 'apostrophe',
  Backquote: 'grave',
  Backslash: 'backslash',
  Comma: 'comma',
  Period: 'period',
  Slash: 'slash',
  ArrowUp: 'up',
  ArrowDown: 'down',
  ArrowLeft: 'left',
  ArrowRight: 'right',
  Home: 'home',
  End: 'end',
  PageUp: 'pageup',
  PageDown: 'pagedown',
  Insert: 'insert',
  Delete: 'delete',
  PrintScreen: 'printscreen',
  AudioVolumeMute: 'mute',
  AudioVolumeDown: 'volumedown',
  AudioVolumeUp: 'volumeup',
  MediaPlayPause: 'playpause',
  MediaTrackNext: 'nexttrack',
  MediaTrackPrevious: 'prevtrack',
}

function keyName(code: string): string | undefined {
  const letter = /^Key([A-Z])$/.exec(code)
  if (letter) return letter[1].toLowerCase()
  const digit = /^(?:Digit|Numpad)(\d)$/.exec(code)
  if (digit) return digit[1]
  const fn = /^F(\d{1,2})$/.exec(code)
  if (fn) return `f${fn[1]}`
  return CODE_NAMES[code]
}

/** Turns a keydown into a combo like `super+shift+b`, or null for a lone modifier/unknown key. */
export function comboFromEvent(e: KeyboardEvent): string | null {
  const key = keyName(e.code)
  if (!key) return null
  const mods = [
    e.metaKey && 'super',
    e.ctrlKey && 'ctrl',
    e.altKey && 'alt',
    e.shiftKey && 'shift',
  ].filter(Boolean)
  return [...mods, key].join('+')
}

export function prettyCombo(combo: string): string {
  return combo
    .split('+')
    .map((part) => (part.length === 1 ? part.toUpperCase() : part[0].toUpperCase() + part.slice(1)))
    .join(' + ')
}
