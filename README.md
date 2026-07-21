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
  - **`src/lib.rs`**: Entry point for Tauri. Manages the embedded LLM engine state and the `generate_response` command.
  - **`src/llm.rs`**: The inference engine powered by `llama-cpp-2`. It automatically loads the downloaded `.gguf` model into the GPU (Metal on Mac) and processes ChatML formatted prompts.
  - **`capabilities/default.json`**: Security policy dictating what OS-level features the frontend is allowed to use.

## 🧠 Local AI Brain (Qwen2.5-3B-Coder)

Martha now runs **100% offline** on your device using a locally embedded Large Language Model.

### How it works:
1. **Auto-Download**: When you run the app for the very first time, a background process will securely download the `qwen2.5-3b-coder-q4_k_m.gguf` model (~2.2GB) directly from HuggingFace to your local application data folder (`~/Library/Application Support/com.hey-martha.dev/models/`).
2. **GPU Acceleration**: The Rust backend uses `llama-cpp-2` with Metal acceleration to load the model entirely into your Mac's GPU memory, allowing for extremely fast, private inference.
3. **No External Servers**: There are no API keys required and absolutely zero data is sent to the internet during conversations.
