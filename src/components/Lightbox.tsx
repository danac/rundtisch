import { useCallback, useEffect } from "react";
import { useTranslation } from "react-i18next";
import type { Artwork } from "../types/portfolio";

interface LightboxProps {
  items: Artwork[];
  index: number;
  onClose: () => void;
  onNavigate: (index: number) => void;
}

export function Lightbox({ items, index, onClose, onNavigate }: LightboxProps) {
  const { t } = useTranslation();
  const item = items[index];

  const goPrev = useCallback(() => {
    onNavigate((index - 1 + items.length) % items.length);
  }, [index, items.length, onNavigate]);

  const goNext = useCallback(() => {
    onNavigate((index + 1) % items.length);
  }, [index, items.length, onNavigate]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
      if (e.key === "ArrowLeft") goPrev();
      if (e.key === "ArrowRight") goNext();
    };
    document.addEventListener("keydown", onKey);
    document.body.style.overflow = "hidden";
    return () => {
      document.removeEventListener("keydown", onKey);
      document.body.style.overflow = "";
    };
  }, [onClose, goPrev, goNext]);

  if (!item) return null;

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label={item.title}
      className="fixed inset-0 z-50 flex flex-col bg-ink/80 p-4 backdrop-blur-sm animate-fade-in sm:p-8"
      onClick={onClose}
    >
      <div className="flex justify-end">
        <button
          type="button"
          onClick={onClose}
          aria-label={t("common.close")}
          className="flex h-11 w-11 items-center justify-center rounded-full bg-white/15 text-2xl text-white transition-colors hover:bg-white/30"
        >
          &times;
        </button>
      </div>

      <div
        className="flex flex-1 items-center justify-center gap-3 overflow-hidden sm:gap-6"
        onClick={(e) => e.stopPropagation()}
      >
        {items.length > 1 && (
          <button
            type="button"
            onClick={goPrev}
            aria-label={t("common.previous")}
            className="hidden h-12 w-12 shrink-0 items-center justify-center rounded-full bg-white/15 text-2xl text-white transition-colors hover:bg-white/30 sm:flex"
          >
            &#8249;
          </button>
        )}

        <figure className="flex max-h-full max-w-4xl flex-col items-center">
          <img
            src={item.image}
            alt={item.alt}
            className="max-h-[70vh] w-auto rounded-soft object-contain shadow-lift animate-fade-in"
          />
          <figcaption className="mt-4 text-center text-white">
            <p className="text-xl font-bold">{item.title}</p>
            <p className="text-sm text-white/70">
              {t(`categories.${item.category}`)}
              {item.year ? ` \u00b7 ${item.year}` : ""}
            </p>
          </figcaption>
        </figure>

        {items.length > 1 && (
          <button
            type="button"
            onClick={goNext}
            aria-label={t("common.next")}
            className="hidden h-12 w-12 shrink-0 items-center justify-center rounded-full bg-white/15 text-2xl text-white transition-colors hover:bg-white/30 sm:flex"
          >
            &#8250;
          </button>
        )}
      </div>

      {/* Mobile prev/next */}
      {items.length > 1 && (
        <div className="flex justify-center gap-4 pt-4 sm:hidden">
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              goPrev();
            }}
            aria-label={t("common.previous")}
            className="flex h-11 w-11 items-center justify-center rounded-full bg-white/15 text-2xl text-white"
          >
            &#8249;
          </button>
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              goNext();
            }}
            aria-label={t("common.next")}
            className="flex h-11 w-11 items-center justify-center rounded-full bg-white/15 text-2xl text-white"
          >
            &#8250;
          </button>
        </div>
      )}
    </div>
  );
}
