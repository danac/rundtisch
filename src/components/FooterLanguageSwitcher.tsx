import { useTranslation } from "react-i18next";

import {
  SUPPORTED_LANGUAGES,
  changeAppLanguage,
  type LanguageCode,
} from "../i18n";

export function FooterLanguageSwitcher() {
  const { t, i18n } = useTranslation();
  const current = (i18n.resolvedLanguage ?? "en") as LanguageCode;

  return (
    <div
      className="flex items-center gap-2"
      role="group"
      aria-label={t("language.label")}
    >
      {SUPPORTED_LANGUAGES.map((lng) => {
        const active = current === lng.code;
        return (
          <button
            key={lng.code}
            type="button"
            onClick={() => void changeAppLanguage(lng.code)}
            aria-current={active ? "true" : undefined}
            aria-label={t("language.switchTo", {
              language: t(`language.${lng.code}`),
            })}
            className={`inline-flex h-7 w-10 items-center justify-center overflow-hidden rounded border transition-opacity focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-coral-500 ${
              active
                ? "border-coral-400 opacity-100"
                : "border-ink/10 opacity-60 hover:opacity-100"
            }`}
          >
            <img
              src={lng.flag}
              alt=""
              className="h-full w-full object-cover"
              width={40}
              height={28}
            />
          </button>
        );
      })}
    </div>
  );
}
