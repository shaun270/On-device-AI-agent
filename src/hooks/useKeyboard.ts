/**
 * useKeyboard — global keyboard shortcut handler.
 *
 * Currently handles Escape:
 *   - If input has text → clear it
 *   - If input is empty → hide the window
 *
 * More shortcuts can be added here later without touching components.
 */

import { useEffect, useCallback } from "react";

interface UseKeyboardOptions {
  inputValue: string;
  onClearInput: () => void;
  onHideWindow: () => void;
}

export function useKeyboard({
  inputValue,
  onClearInput,
  onHideWindow,
}: UseKeyboardOptions) {
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (inputValue.trim().length > 0) {
          onClearInput();
        } else {
          onHideWindow();
        }
      }
    },
    [inputValue, onClearInput, onHideWindow]
  );

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);
}
