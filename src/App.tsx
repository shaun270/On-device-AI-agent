import { FormEvent, useState } from "react";
import "./App.css";
import { APP_NAME, APP_PLACEHOLDER } from "./constants";

function App() {
  const [input, setInput] = useState("");
  const [response, setResponse] = useState("");

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    const trimmed = input.trim();
    if (!trimmed) return;
    setResponse(trimmed);
    setInput("");
  }

  return (
    <div className="bar" data-tauri-drag-region>
      <header className="brand" data-tauri-drag-region>
        <span className="brand-mark" data-tauri-drag-region>{APP_NAME}</span>
      </header>

      <form className="input-row" onSubmit={handleSubmit}>
        <input
          type="text"
          value={input}
          onChange={(e) => setInput(e.currentTarget.value)}
          placeholder={APP_PLACEHOLDER}
          aria-label={`Ask ${APP_NAME}`}
          autoFocus
        />
        <button type="submit">Send</button>
      </form>

      <div className="response" aria-live="polite">
        {response ? response : <span className="placeholder">Response appears here</span>}
      </div>
    </div>
  );
}

export default App;
