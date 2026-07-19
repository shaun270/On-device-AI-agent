# Martha: AI Agent Desktop App

Martha is a professional-grade, multi-session AI assistant built as a cross-platform desktop application. It features a sleek glassmorphic UI, a toggleable global hotkey, and a persistent multi-chat session architecture.

## 🚀 Tech Stack

- **Frontend**: React + TypeScript + Vite
- **Styling**: Vanilla CSS (Glassmorphism design system)
- **Backend**: Rust + Tauri v2
- **Persistence**: LocalStorage (Sessions & Settings)

## ✨ Key Features

- **Multi-Session Chat**: Create, rename, and manage multiple persistent chat histories.
- **Auto-Titling**: Chat sessions automatically adopt the first message as their title.
- **Global Hotkey**: Press `Ctrl+M` (or customize it) from anywhere on your computer to instantly summon or hide Martha.
- **Glassmorphic UI**: Premium translucent dark-mode aesthetic.
- **Minimize-to-Dock**: Operates reliably in the background without disappearing from your taskbar/dock.

## 🛠️ Setup & Installation

### Prerequisites
You need the standard Tauri prerequisites installed on your system:
- [Node.js](https://nodejs.org) (v18+)
- [Rust](https://www.rust-lang.org/tools/install)
- [Cargo](https://doc.rust-lang.org/cargo/)

### Quickstart

1. **Install dependencies:**
   ```bash
   npm install
   ```

2. **Run in development mode:**
   ```bash
   npm run tauri dev
   ```
   *This starts the Vite dev server and the Rust backend. Any changes to the UI or Rust code will automatically hot-reload.*

3. **Build for production:**
   ```bash
   npm run tauri build
   ```
   *This creates an optimized, bundled executable for your operating system in `src-tauri/target/release/bundle/`.*

## ⚙️ App Architecture

- **`src/`**: Contains the React frontend.
  - **`components/`**: Modular UI pieces (BrandBar, InputArea, ChatLog, SessionDrawer, SettingsPanel).
  - **`hooks/`**: Business logic separated from the UI (`useChatSessions`, `useWindowControls`, `useSettings`).
- **`src-tauri/`**: Contains the Rust backend.
  - **`src/lib.rs`**: Entry point for Tauri. Handles native window interactions and the placeholder `echo_message` command for AI responses.
  - **`capabilities/default.json`**: Security policy dictating what OS-level features the frontend is allowed to use.

## 🔮 Next Steps for Developers

To wire up a real LLM (like OpenAI or Anthropic):
1. Open `src-tauri/src/lib.rs`.
2. Locate the `echo_message` command.
3. Replace the `tokio::time::sleep` mock with a real HTTP request to your preferred AI provider.
