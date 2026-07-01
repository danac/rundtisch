import type { LanguageCode } from "../i18n/languages";
import { apiGet, hasBackend } from "./apiClient";

export type TranslationBundle = Record<string, unknown>;

const bundledFallback: Record<LanguageCode, () => Promise<TranslationBundle>> = {
  en: () => import("../i18n/locales/en/translation.json").then((m) => m.default),
  fr: () => import("../i18n/locales/fr/translation.json").then((m) => m.default),
};

/**
 * Loads UI translations for a locale. Static mode fetches served JSON from
 * `/locales/{lng}/translation.json`; backend mode uses `GET /translations/{lng}`.
 * Falls back to bundled locale files when fetch fails.
 */
export async function getTranslations(
  lng: LanguageCode,
  signal?: AbortSignal,
): Promise<TranslationBundle> {
  try {
    if (hasBackend) {
      return await apiGet<TranslationBundle>(`/translations/${lng}`, signal);
    }
    const res = await fetch(`/locales/${lng}/translation.json`, { signal });
    if (!res.ok) {
      throw new Error(`Failed to load /locales/${lng}/translation.json`);
    }
    return (await res.json()) as TranslationBundle;
  } catch {
    return bundledFallback[lng]();
  }
}
