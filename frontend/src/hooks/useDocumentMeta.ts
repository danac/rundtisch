import { useEffect } from "react";
import { useTranslation } from "react-i18next";

/** Keeps document title, description meta, and html lang in sync with i18n. */
export function useDocumentMeta() {
  const { t, i18n } = useTranslation();

  useEffect(() => {
    document.documentElement.lang = i18n.resolvedLanguage ?? "en";
    document.title = t("meta.title");

    let description = document.querySelector('meta[name="description"]');
    if (!description) {
      description = document.createElement("meta");
      description.setAttribute("name", "description");
      document.head.appendChild(description);
    }
    description.setAttribute("content", t("meta.description"));
  }, [t, i18n.resolvedLanguage]);
}
