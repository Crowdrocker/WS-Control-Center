# WS Control Center

A local-first workstation command deck for Linux gaming and creator workflows. The first milestone is a Tauri v2 desktop shell with a dark industrial interface and navigable module placeholders.

## Development

Requirements: Node.js, npm, Rust, and the platform dependencies required by Tauri.

```sh
npm install
npm run tauri dev
```

Create a production frontend build with `npm run build`, or package the desktop app with `npm run tauri build`.

## Current scope

The interface includes placeholder sections for Mod Organizer 2 / Proton, Steam and non-Steam games, app launching, Ollama voice control, GitHub dotfile sync, system tools, and OBS Studio. These are not connected to system services yet; the app currently makes no system changes and does not probe local applications or devices.