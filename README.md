<picture>
  <source media="(prefers-color-scheme: dark)" srcset="logo-dark.png">
  <img alt="Conduit" src="logo.png">
</picture>

[![Buy Me a Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-deid84-FFDD00?style=for-the-badge&logo=buy-me-a-coffee&logoColor=black)](https://buymeacoffee.com/deid84)
[![License](https://img.shields.io/badge/license-MIT%20%7C%20Apache--2.0-blue?style=for-the-badge)](LICENSE-MIT)

A serial/TCP/UDP monitor and protocol gateway — alternative to Hercules.

Runs in two modes:
- **GUI** — desktop app (Tauri + Svelte 5) for interactive monitoring
- **API** — REST + WebSocket gateway (Axum) for scripting and automation, with embedded web UI

## Features

- **Serial connections** — RS-232 and RS-485; configurable baud rate, data bits, parity, stop bits, flow control; DTR/RTS control
- **TCP connections** — client mode and server (listener) mode
- **UDP connections** — bind to a local port, send/receive datagrams
- **Multi-tab UI** — open multiple connections simultaneously, each in its own tab
- **Timestamped frame log** — every RX/TX frame is logged with direction and timestamp
- **WebSocket live stream** — subscribe to `/api/connections/{id}/stream` for raw bytes and decoded frames in real time
- **REST API** — open/close connections, send raw data, list serial ports, set DTR/RTS

### Modbus

- **Protocols**: Modbus RTU, ASCII, and TCP
- **Request builder**: function codes FC01 (Read Coils), FC02 (Read Discrete Inputs), FC03 (Read Holding Registers), FC04 (Read Input Registers), FC05 (Write Single Coil), FC06 (Write Single Register), FC15 (Write Multiple Coils), FC16 (Write Multiple Registers)
- **Response decoder**: automatic parsing and display of register/coil data
- **Display formats**: hexadecimal, uint16, int16, uint32, int32, float32, string
- **Byte swap and word swap** options for multi-register values
- **Configurable timeout and retry** settings, saved per session

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) ≥ 18
- Tauri CLI: `cargo install tauri-cli --version "^2"`
- On Windows: [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (pre-installed on Windows 10/11)

## Setup

```sh
git clone https://github.com/deid84/conduit
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

### API gateway (Axum REST + WebSocket + UI)

Build the frontend first, then start the server:

```sh
cd frontend && npm run build && cd ..
cargo run --bin conduit-api
# Listening on 0.0.0.0:3000
# Open http://localhost:3000 in a browser
```

The binary serves `frontend/dist` as static files with an `index.html`
fallback for client-side routing. To override the path:

```sh
# via environment variable
CONDUIT_STATIC_DIR=/path/to/dist cargo run --bin conduit-api

# or via .env file in the working directory
echo 'CONDUIT_STATIC_DIR=/path/to/dist' > .env
cargo run --bin conduit-api
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

# API gateway — optimized binary
cargo build --bin conduit-api --release
```

## Workspace structure

```
conduit/
├── crates/
│   ├── conduit-core/   # transport logic: serial, TCP, UDP
│   ├── conduit-api/    # Axum REST + WebSocket gateway (binary: conduit-api)
│   └── conduit-tauri/  # Tauri desktop app (binary: conduit)
└── frontend/           # Svelte 5 + Vite
```

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE) at your option.
