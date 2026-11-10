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

## Production build

```sh
(cd web && npm run build)
cargo build --release -p soundboard-server
./target/release/soundboard-server   # UI is embedded; open http://<this-pc-ip>:7878
```
