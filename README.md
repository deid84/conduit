# Conduit

A serial/TCP/UDP monitor and protocol gateway — alternative to Hercules.

Runs in two modes:
- **GUI** — desktop app (Tauri + Svelte 5) for interactive monitoring
- **Headless** — REST + WebSocket API gateway (Axum) for scripting and automation

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) ≥ 18
- Tauri CLI: `cargo install tauri-cli --version "^2"`
- On Windows: [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (pre-installed on Windows 10/11)

## Setup

```sh
git clone https://github.com/your-handle/conduit
cd conduit/frontend
npm install
cd ..
```

## Running

### GUI (Tauri desktop app)

```sh
cd crates/conduit-tauri
cargo tauri dev
```

Tauri starts the Vite dev server automatically and opens the app window.

### Headless gateway (Axum REST + WebSocket)

```sh
cargo run -p conduit-headless
# Listening on 0.0.0.0:3000
```

### Frontend only (Vite dev server)

```sh
cd frontend
npm run dev
# http://localhost:5173
```

## Building for production

```sh
# GUI — produces a native installer
cd crates/conduit-tauri
cargo tauri build

# Headless — optimized binary
cargo build --release -p conduit-headless
```

## Workspace structure

```
conduit/
├── crates/
│   ├── conduit-core/       # transport logic: serial, TCP, UDP
│   ├── conduit-headless/   # Axum REST + WebSocket gateway
│   └── conduit-tauri/      # Tauri desktop app
└── frontend/               # Svelte 5 + Vite
```

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE) at your option.
