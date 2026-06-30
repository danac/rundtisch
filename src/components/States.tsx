import { useTranslation } from "react-i18next";

export function LoadingGrid({ count = 6 }: { count?: number }) {
  const { t } = useTranslation();
  return (
    <div>
      <span className="sr-only">{t("common.loading")}</span>
      <div className="columns-1 gap-5 sm:columns-2 lg:columns-3 [&>*]:mb-5">
        {Array.from({ length: count }).map((_, i) => (
          <div
            key={i}
            className="animate-pulse rounded-blob bg-paper"
            style={{ height: `${220 + (i % 3) * 60}px` }}
          />
        ))}
      </div>
    </div>
  );
}

export function ErrorState({ onRetry }: { onRetry?: () => void }) {
  const { t } = useTranslation();
  return (
    <div className="mx-auto max-w-md rounded-blob bg-coral-50 p-10 text-center">
      <p className="text-2xl font-bold text-coral-600">{t("common.errorTitle")}</p>
      <p className="mt-2 text-ink-soft">{t("common.errorBody")}</p>
      {onRetry && (
        <button type="button" onClick={onRetry} className="btn-primary mt-6">
          {t("common.retry")}
        </button>
      )}
    </div>
  );
}

export function EmptyState({ message }: { message: string }) {
  return (
    <div className="mx-auto max-w-md rounded-blob bg-paper p-10 text-center text-lg text-ink-soft">
      {message}
    </div>
  );
}
