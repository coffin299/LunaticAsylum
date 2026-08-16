import type { ThemePreference } from "@lunatic-asylum/shared";

const STORAGE_KEY = "lunatic-asylum.theme";

export function readThemePreference(): ThemePreference {
  const raw = localStorage.getItem(STORAGE_KEY);
  if (raw === "light" || raw === "dark" || raw === "system") {
    return raw;
  }
  return "system";
}

export function saveThemePreference(pref: ThemePreference): void {
  localStorage.setItem(STORAGE_KEY, pref);
}

function systemIsDark(): boolean {
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

/** 単色テーマを data-theme に反映（グラデーションなし） */
export function applyTheme(pref: ThemePreference): void {
  const resolved = pref === "system" ? (systemIsDark() ? "dark" : "light") : pref;
  document.documentElement.setAttribute("data-theme", resolved);
}

export function initTheme(): ThemePreference {
  const pref = readThemePreference();
  applyTheme(pref);
  window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
    if (readThemePreference() === "system") {
      applyTheme("system");
    }
  });
  return pref;
}
