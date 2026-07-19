/**
 * useSettings — persists AppSettings to localStorage.
 *
 * Pattern: read once on mount, write on every change.
 * Merges with DEFAULT_SETTINGS so new fields added later have safe fallbacks.
 */

import { useState, useEffect } from "react";
import type { AppSettings } from "../types";
import { STORAGE_KEYS, DEFAULT_SETTINGS } from "../constants";

function loadSettings(): AppSettings {
  try {
    const raw = localStorage.getItem(STORAGE_KEYS.SETTINGS);
    // Spread DEFAULT_SETTINGS first so any new keys we add always exist.
    return raw
      ? { ...DEFAULT_SETTINGS, ...(JSON.parse(raw) as Partial<AppSettings>) }
      : { ...DEFAULT_SETTINGS };
  } catch {
    return { ...DEFAULT_SETTINGS };
  }
}

export function useSettings() {
  const [settings, setSettings] = useState<AppSettings>(loadSettings);

  useEffect(() => {
    localStorage.setItem(STORAGE_KEYS.SETTINGS, JSON.stringify(settings));
  }, [settings]);

  /** Type-safe helper: updateSetting('opacity', 0.8) */
  function updateSetting<K extends keyof AppSettings>(
    key: K,
    value: AppSettings[K]
  ) {
    setSettings((prev) => ({ ...prev, [key]: value }));
  }

  return { settings, updateSetting };
}
