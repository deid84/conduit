<picture>
  <source media="(prefers-color-scheme: dark)" srcset="logo-dark.png">
  <img alt="Conduit" src="logo.png">
</picture>

[![Buy Me a Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-deid84-FFDD00?style=for-the-badge&logo=buy-me-a-coffee&logoColor=black)](https://buymeacoffee.com/deid84)
[![License](https://img.shields.io/badge/license-MIT%20%7C%20Apache--2.0-blue?style=for-the-badge)](LICENSE-MIT)

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

### Headless gateway (Axum REST + WebSocket + UI)

Build the frontend first, then start the server:

```sh
cd frontend && npm run build && cd ..
cargo run -p conduit-headless
# Listening on 0.0.0.0:3000
# Open http://localhost:3000 in a browser
```

The binary serves `frontend/dist` as static files with an `index.html`
fallback for client-side routing. To override the path:

```sh
# via environment variable
CONDUIT_STATIC_DIR=/path/to/dist cargo run -p conduit-headless

# or via .env file in the working directory
echo 'CONDUIT_STATIC_DIR=/path/to/dist' > .env
cargo run -p conduit-headless
```

### Frontend only (Vite dev server)

```sh
cd frontend
npm run dev
# http://localhost:8419
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
