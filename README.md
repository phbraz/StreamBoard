# Soundboard

A LAN stream deck: press buttons on any PC, laptop or phone, and the target PC
plays sounds (into Discord/OBS through a virtual mic) or runs actions (hotkeys,
mic mute). See [docs/DESIGN.md](docs/DESIGN.md).

## Layout

- `server/`: Rust server (axum): HTTP API, WebSocket, audio, actions. Serves the built UI.
- `web/`: React + Vite + Tailwind UI.
- `config/`: example config files.

## Development

```sh
# terminal 1: server on :7878
cargo run -p soundboard-server

# terminal 2: UI with hot reload on :5173 (proxies /api and /ws to :7878)
cd web && npm install && npm run dev
```

Open `http://localhost:5173`, or `http://<this-pc-ip>:5173` from your phone.

Regenerate the TypeScript protocol types after changing `server/src/protocol.rs`:

```sh
cargo test -p soundboard-server
```

## Sounds and hotkeys

- **+ Sound** on the board uploads mp3, ogg, wav, flac or m4a files to the server.
  Files copied straight into `~/.local/share/soundboard/sounds` (or `audio.sounds_dir`)
  show up within a few seconds too.
- **+ Hotkey** adds a button that presses a key combo on the server, like
  `super+shift+b`. It uses `ydotool`, so `ydotoold` must be running
  (`systemctl --user enable --now ydotool`).
- **Edit** lets you rename, re-key or delete buttons. Deleting a sound also deletes its file.

The board is saved in `~/.local/share/soundboard/board.json`.

## Reaching it from other devices

1. Allow the ports from the LAN (ufw is active on Omarchy):
   ```sh
   sudo ufw allow from 192.168.1.0/24 to any port 5173 proto tcp comment 'soundboard dev'
   sudo ufw allow from 192.168.1.0/24 to any port 7878 proto tcp comment 'soundboard'
   ```
2. On Android, Chrome and Brave need the **Nearby devices** / **Local network**
   permission (Settings → Apps → Chrome → Permissions), then a browser restart.
   Without it, requests to LAN addresses are silently dropped. The stock browser
   usually works without it.
3. If it still fails: turn off "Always use secure connections" (Chrome) or
   "Upgrade connections to HTTPS" (Brave), pause VPN/ad-block DNS apps, and
   check the router for AP/client isolation.

## Production build

```sh
(cd web && npm run build)
cargo build --release -p soundboard-server
./target/release/soundboard-server   # UI is embedded; open http://<this-pc-ip>:7878
```
