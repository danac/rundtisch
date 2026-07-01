import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import LanguageDetector from "i18next-browser-languagedetector";

import { getTranslations } from "../services/translationService";
import {
  LANGUAGE_CODES,
  type LanguageCode,
} from "./languages";

export { SUPPORTED_LANGUAGES, type LanguageCode } from "./languages";

const loaded = new Set<string>();

async function loadLanguage(lng: LanguageCode): Promise<void> {
  if (loaded.has(lng)) return;
  const bundle = await getTranslations(lng);
  i18n.addResourceBundle(lng, "translation", bundle, true, true);
  loaded.add(lng);
}

export async function changeAppLanguage(lng: LanguageCode): Promise<void> {
  await loadLanguage(lng);
  await i18n.changeLanguage(lng);
}

export async function initI18n(): Promise<void> {
  await i18n
    .use(LanguageDetector)
    .use(initReactI18next)
    .init({
      fallbackLng: "en",
      supportedLngs: LANGUAGE_CODES,
      interpolation: {
        escapeValue: false,
      },
      detection: {
        order: ["localStorage", "navigator", "htmlTag"],
        caches: ["localStorage"],
      },
    });

  const initial = (i18n.resolvedLanguage ?? "en") as LanguageCode;
  await loadLanguage(initial);

  const other = LANGUAGE_CODES.find((code) => code !== initial);
  if (other) {
    void loadLanguage(other);
  }
}

export default i18n;
