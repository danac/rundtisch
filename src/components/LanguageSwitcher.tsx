import { useTranslation } from "react-i18next";
import { SUPPORTED_LANGUAGES } from "../i18n";

/**
 * Language selector. Hidden while only one locale is available, but fully
 * wired so adding a locale in `src/i18n` makes it appear automatically.
 */
export function LanguageSwitcher({ className = "" }: { className?: string }) {
  const { i18n } = useTranslation();

  if (SUPPORTED_LANGUAGES.length < 2) return null;

  return (
    <label className={`inline-flex items-center gap-2 ${className}`}>
      <span className="sr-only">Language</span>
      <select
        value={i18n.resolvedLanguage}
        onChange={(e) => void i18n.changeLanguage(e.target.value)}
        className="rounded-full border-2 border-ink/10 bg-white/70 px-3 py-1.5 text-sm font-semibold text-ink focus:border-coral-300 focus:outline-none"
      >
        {SUPPORTED_LANGUAGES.map((lng) => (
          <option key={lng.code} value={lng.code}>
            {lng.label}
          </option>
        ))}
      </select>
    </label>
  );
}
