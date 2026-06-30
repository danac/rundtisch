import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { Gallery } from "../components/Gallery";
import { Section, SectionHeading } from "../components/Section";
import { LoadingGrid, ErrorState, EmptyState } from "../components/States";
import { useAsyncData } from "../hooks/useAsyncData";
import { getArtworks } from "../services/portfolioService";
import type { ArtworkCategory } from "../types/portfolio";

type Filter = "all" | ArtworkCategory;

const filters: Filter[] = ["all", "birds", "lettering", "children"];

export function Portfolio() {
  const { t } = useTranslation();
  const { data, loading, error, reload } = useAsyncData(getArtworks);
  const [filter, setFilter] = useState<Filter>("all");

  const visible = useMemo(() => {
    if (!data) return [];
    return filter === "all" ? data : data.filter((a) => a.category === filter);
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
        {loading && <LoadingGrid count={9} />}
        {error && <ErrorState onRetry={reload} />}
        {data &&
          (visible.length > 0 ? (
            <Gallery items={visible} />
          ) : (
            <EmptyState message={t("portfolio.empty")} />
          ))}
      </div>
    </Section>
  );
}
