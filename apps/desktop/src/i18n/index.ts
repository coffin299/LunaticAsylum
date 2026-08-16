import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import type { LocalePreference } from "@lunatic-asylum/shared";
import { en, ja } from "./locales";

const STORAGE_KEY = "lunatic-asylum.locale";

export function readLocalePreference(): LocalePreference {
  const raw = localStorage.getItem(STORAGE_KEY);
  if (raw === "ja" || raw === "en" || raw === "system") {
    return raw;
  }
  return "system";
}

export function resolveLocale(pref: LocalePreference): "ja" | "en" {
  if (pref === "ja" || pref === "en") {
    return pref;
  }
  const nav = navigator.language.toLowerCase();
  return nav.startsWith("ja") ? "ja" : "en";
}

export function saveLocalePreference(pref: LocalePreference): void {
  localStorage.setItem(STORAGE_KEY, pref);
}

void i18n.use(initReactI18next).init({
  resources: {
    ja: { translation: ja },
    en: { translation: en },
  },
  lng: resolveLocale(readLocalePreference()),
  fallbackLng: "en",
  interpolation: { escapeValue: false },
});

export default i18n;
