# Hey Martha

On-device AI agent for macOS. Phase 1 scaffold: floating always-on-top bar with text echo.

## Develop

Requires [rustup](https://rustup.rs/) stable on your `PATH` (ahead of any Homebrew `rustc`).

```bash
export PATH="$HOME/.cargo/bin:$PATH"
npm install
npm run tauri dev
```

Type a message and press Send — it echoes into the response area. No agent/tools yet.
