/** Supported locale codes and flag assets for the language switcher. */
export const SUPPORTED_LANGUAGES = [
  { code: "en", flag: "/images/flags/gb.svg" },
  { code: "fr", flag: "/images/flags/fr.svg" },
] as const;

export type LanguageCode = (typeof SUPPORTED_LANGUAGES)[number]["code"];

export const LANGUAGE_CODES = SUPPORTED_LANGUAGES.map((l) => l.code);
