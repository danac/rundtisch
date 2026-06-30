import { useTranslation } from "react-i18next";

export function ConstructionBanner() {
  const { t } = useTranslation();

  return (
    <div
      role="status"
      className="border-b border-ink/5 bg-sun-300/20 px-4 py-2 text-center text-xs font-medium text-ink-soft sm:text-sm"
    >
      {t("banner.construction")}
    </div>
  );
}
