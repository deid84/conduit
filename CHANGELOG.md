
# Changelog

All notable changes to Conduit.
## [0.1.0] — 2026-06-29

### Bug Fixes
- **readme**: Clarify frontend npm install step
- **tauri**: Use cwd field for beforeDevCommand instead of npm --prefix
- **tauri**: Generate full icon set for Linux build
- CORS + new-connection tab indicator
- **tauri**: Set socket non-blocking before from_std conversion
- Propagate backend error to frontend + highlight active + button
- **terminal**: Remove overlay, move ASCII/HEX toggle to SendBar
- **sendbar**: Lock LINE/RAW selector entirely when HEX input is active
- **layout**: Set #app height:100% so flex column fills full viewport
- **sendbar**: Align toggle buttons height with input and Send button
- **ui**: Uniform heights + move Clear button to SendBar
- **sendbar**: Style Clear button as destructive (red border + hover)
- **logging**: Show inline dialog when File System Access API unavailable
- **ui**: Override native macOS select styling with custom chevron


### Chore
- Initialize Rust workspace
- Change dev port from 5173 to 8419
- Add brand icons and logo
- Add GitHub Sponsors funding config and macOS Tauri schema
- Add Buy Me a Coffee to funding options


### Documentation
- Document headless static serving and CONDUIT_STATIC_DIR override
- Add Buy Me a Coffee badge to README
- Add Buy Me a Coffee badge to README
- Update README for conduit-api rename and new binary names


### Features
- **core**: Add conduit-core library stub
- **headless**: Add Axum REST/WebSocket gateway skeleton
- **tauri**: Add Tauri v2 desktop app skeleton
- **frontend**: Add Svelte 5 + Vite 6 frontend scaffold
- **core**: Implement Connection abstraction and serial/tcp/udp transports
- **headless**: Implement REST + WebSocket API gateway
- **frontend**: Add Tailwind v4 + shadcn-svelte scaffold
- **frontend**: Add main UI — tabs, terminal, send bar, connection form
- **frontend**: Wire frontend to Axum API
- **frontend**: Replace terminal placeholder with xterm.js
- **tauri**: Wire Axum server on random loopback port + inject API URL
- **frontend**: Add raw (minicom-like) terminal mode
- **frontend**: Add light/dark/system theme with Catppuccin Latte+Mocha
- **frontend**: Add hex view + hex send
- **sendbar**: Disable RAW mode when HEX input is active
- **frontend**: Add MacroBar with preset commands and file send
- Add Clear button to wipe terminal display
- **terminal**: Right-click context menu with Clear option
- **theme**: Replace Catppuccin with brand palette (Bone/Graphite/Amber)
- **headless**: Serve frontend/dist as static files with env override
- **ui**: Settings panel + session log recording/export
- **logging**: Show OS save dialog before starting REC
- **signals**: Serial line signals, DTR/RTS control, and connection profiles
- **terminal**: Copy/paste via context menu and keyboard shortcuts
- **profiles**: Export to JSON file and import from JSON file


### Refactoring
- Rename conduit-headless → conduit-api, conduit-tauri binary → conduit


### Styling
- Fixed logo size on readme.md
- **dark**: Switch dark bg to Cinder (#252019) for better readability

