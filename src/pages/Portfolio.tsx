import { useMemo } from "react";
import { useSearchParams } from "react-router-dom";
import { useTranslation } from "react-i18next";

import { CollectionCard } from "../components/CollectionCard";
import { Section, SectionHeading } from "../components/Section";
import { LoadingGrid, ErrorState, EmptyState } from "../components/States";
import { useAsyncData } from "../hooks/useAsyncData";
import { getCollections } from "../services/portfolioService";
import type { ArtworkCategory } from "../types/portfolio";

type Filter = "all" | ArtworkCategory;

const filters: Filter[] = ["all", "birds", "lettering", "children"];

const CATEGORY_VALUES: ArtworkCategory[] = ["birds", "lettering", "children"];

function parseCategoryParam(value: string | null): Filter {
  if (value && CATEGORY_VALUES.includes(value as ArtworkCategory)) {
    return value as ArtworkCategory;
  }
  return "all";
}

export function Portfolio() {
  const { t } = useTranslation();
  const [searchParams, setSearchParams] = useSearchParams();
  const { data, loading, error, reload } = useAsyncData(getCollections);

  const filter = parseCategoryParam(searchParams.get("category"));

  const setFilter = (next: Filter) => {
    if (next === "all") {
      setSearchParams({}, { replace: true });
    } else {
      setSearchParams({ category: next }, { replace: true });
    }
  };

  const visible = useMemo(() => {
    if (!data) return [];
    return filter === "all"
      ? data
      : data.filter((c) => c.category === filter);
  }, [data, filter]);

  return (
    <Section>
      <SectionHeading
        eyebrow={t("portfolio.eyebrow")}
        title={t("portfolio.title")}
        subtitle={t("portfolio.subtitle")}
      />

      <div className="mt-10 flex flex-wrap justify-center gap-2">
        {filters.map((f) => (
          <button
            key={f}
            type="button"
            onClick={() => setFilter(f)}
            className={`pill border-2 transition-colors ${
              filter === f
                ? "border-coral-500 bg-coral-500 text-white"
                : "border-ink/10 bg-white text-ink hover:border-coral-300"
            }`}
          >
            {t(`portfolio.filters.${f}`)}
          </button>
        ))}
      </div>

      <div className="mt-12">
        {loading && <LoadingGrid count={6} />}
        {error && <ErrorState onRetry={reload} />}
        {data &&
          (visible.length > 0 ? (
            <div className="grid grid-cols-1 gap-5 sm:grid-cols-2 lg:grid-cols-3">
              {visible.map((collection) => (
                <CollectionCard key={collection.id} collection={collection} />
              ))}
            </div>
          ) : (
            <EmptyState message={t("portfolio.empty")} />
          ))}
      </div>
    </Section>
  );
}
