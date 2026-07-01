import { useCallback } from "react";
import { Link, useParams } from "react-router-dom";
import { useTranslation } from "react-i18next";

import { formatPrice } from "../components/ProductCard";
import { Section } from "../components/Section";
import { LoadingGrid, ErrorState, EmptyState } from "../components/States";
import { useAsyncData } from "../hooks/useAsyncData";
import { getProductById } from "../services/merchService";

export function ProductPage() {
  const { t } = useTranslation();
  const { id } = useParams<{ id: string }>();

  const fetcher = useCallback(
    (signal: AbortSignal) => {
      if (!id) return Promise.resolve(undefined);
      return getProductById(id, signal);
    },
    [id],
  );

  const { data: product, loading, error, reload } = useAsyncData(fetcher);

  if (!id) {
    return (
      <Section>
        <EmptyState message={t("merch.productNotFound")} />
      </Section>
    );
  }

  return (
    <Section>
      <div className="flex items-center gap-4">
        <Link
          to="/shop"
          className="ml-auto inline-flex shrink-0 items-center gap-2 text-sm font-semibold text-coral-600 transition-colors hover:text-coral-500"
        >
          <span aria-hidden>&#8592;</span>
          {t("merch.backToShop")}
        </Link>
      </div>

      {loading && (
        <div className="mt-12">
          <LoadingGrid count={1} />
        </div>
      )}

      {error && (
        <div className="mt-12">
          <ErrorState onRetry={reload} />
        </div>
      )}

      {product === undefined && !loading && !error && (
        <div className="mt-12">
          <EmptyState message={t("merch.productNotFound")} />
        </div>
      )}

      {product && (
        <div className="mt-8 grid gap-10 lg:grid-cols-2 lg:items-start">
          <figure className="overflow-hidden rounded-blob bg-white shadow-soft">
            <img
              src={product.image}
              alt={product.alt}
              className="w-full object-contain animate-fade-in"
            />
          </figure>

          <div className="animate-fade-up">
            {product.soldOut && (
              <span className="pill bg-ink/80 text-white">
                {t("merch.soldOut")}
              </span>
            )}
            <h1 className="mt-4 text-3xl font-bold sm:text-4xl">
              {product.title}
            </h1>
            <p className="mt-4 text-lg text-ink-soft">{product.description}</p>
            <p className="mt-6 text-3xl font-bold text-coral-600">
              {formatPrice(product)}
            </p>
            <a
              href={product.url}
              target="_blank"
              rel="noreferrer noopener"
              aria-disabled={product.soldOut}
              className={`btn mt-8 px-8 py-3 ${
                product.soldOut
                  ? "pointer-events-none bg-ink/10 text-ink-soft"
                  : "bg-teal-500 text-white hover:bg-teal-600"
              }`}
            >
              {product.soldOut ? t("merch.soldOut") : t("merch.buyNow")}
            </a>
          </div>
        </div>
      )}
    </Section>
  );
}
