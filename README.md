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

The Steam library module can scan standard Linux Steam locations and their configured libraries when requested. It reads installed app manifests only; it does not launch games or modify files. The other sections are UI placeholders and are not connected to system services.

Run the focused Rust parser tests with `cargo test --manifest-path src-tauri/Cargo.toml`.