import { useCallback } from "react";
import { Link, useParams } from "react-router-dom";
import { useTranslation } from "react-i18next";

import { Gallery } from "../components/Gallery";
import { Section } from "../components/Section";
import { LoadingGrid, ErrorState, EmptyState } from "../components/States";
import { useAsyncData } from "../hooks/useAsyncData";
import { getCollectionBySlug } from "../services/portfolioService";

export function CollectionPage() {
  const { t } = useTranslation();
  const { slug } = useParams<{ slug: string }>();

  const fetcher = useCallback(
    (signal: AbortSignal) => {
      if (!slug) return Promise.resolve(undefined);
      return getCollectionBySlug(slug, signal);
    },
    [slug],
  );

  const { data: collection, loading, error, reload } = useAsyncData(fetcher);

  if (!slug) {
    return (
      <Section>
        <EmptyState message={t("portfolio.collectionNotFound")} />
      </Section>
    );
  }

  return (
    <Section>
      <Link
        to="/portfolio"
        className="inline-flex items-center gap-2 text-sm font-semibold text-coral-600 transition-colors hover:text-coral-500"
      >
        <span aria-hidden>&#8592;</span>
        {t("portfolio.backToPortfolio")}
      </Link>

      {loading && (
        <div className="mt-12">
          <LoadingGrid count={6} />
        </div>
      )}

      {error && (
        <div className="mt-12">
          <ErrorState onRetry={reload} />
        </div>
      )}

      {collection === undefined && !loading && !error && (
        <div className="mt-12">
          <EmptyState message={t("portfolio.collectionNotFound")} />
        </div>
      )}

      {collection && (
        <>
          <header className="mt-8 max-w-2xl animate-fade-up">
            <span className="pill bg-teal-50 text-teal-600">
              {t(`categories.${collection.category}`)}
            </span>
            <h1 className="mt-4 text-3xl font-bold sm:text-4xl">
              {collection.title}
            </h1>
            <p className="mt-4 text-lg text-ink-soft">
              {collection.description}
            </p>
            <p className="mt-2 text-sm text-ink-soft">
              {t("portfolio.pieceCount", {
                count: collection.artworks.length,
              })}
            </p>
          </header>

          <div className="mt-12">
            <Gallery items={collection.artworks} />
          </div>
        </>
      )}
    </Section>
  );
}
