/**
 * useWindowControls — Tauri window API wrappers.
 *
 * Close button uses MINIMIZE (not hide) so the window stays in the dock/taskbar
 * and can be restored.
 *
 * Global Hotkey is registered here to toggle window visibility.
 */

import { useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { register, unregister } from "@tauri-apps/plugin-global-shortcut";
import type { AppSettings } from "../types";

export function useWindowControls(settings: AppSettings) {
  // Keep CSS opacity variable in sync
  useEffect(() => {
    document.documentElement.style.setProperty(
      "--bg-alpha",
      String(settings.opacity)
    );
  }, [settings.opacity]);

  // Keep alwaysOnTop in sync
  useEffect(() => {
    getCurrentWindow().setAlwaysOnTop(settings.alwaysOnTop).catch(console.error);
  }, [settings.alwaysOnTop]);

  // Global Hotkey registration (Toggle window state)
  useEffect(() => {
    if (!settings.globalHotkey) return;

    let registered = false;

    async function setupHotkey() {
      try {
        await register(settings.globalHotkey, async (event) => {
          if (event.state === "Pressed") {
            const win = getCurrentWindow();
            const isVisible = await win.isVisible();
            const isFocused = await win.isFocused();
            const isMinimized = await win.isMinimized();

            if (isVisible && isFocused && !isMinimized) {
              await win.minimize();
            } else {
              if (isMinimized) await win.unminimize();
              await win.show();
              await win.setFocus();
            }
          }
        });
        registered = true;
      } catch (err) {
        console.error("Failed to register global hotkey:", err);
      }
    }

    setupHotkey();

    return () => {
      if (registered) {
        unregister(settings.globalHotkey).catch(console.error);
      }
    };
  }, [settings.globalHotkey]);

  /** Minimize to dock/taskbar */
  async function minimizeWindow() {
    try {
      await getCurrentWindow().minimize();
    } catch (e) {
      console.error("Failed to minimize window:", e);
    }
  }

  return { minimizeWindow };
}
