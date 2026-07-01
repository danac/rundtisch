import { useTranslation } from "react-i18next";

import { ProductCard } from "../components/ProductCard";
import { Section, SectionHeading } from "../components/Section";
import { LoadingGrid, ErrorState, EmptyState } from "../components/States";
import { useAsyncData } from "../hooks/useAsyncData";
import { getProducts } from "../services/merchService";

export function Merch() {
  const { t } = useTranslation();
  const { data, loading, error, reload } = useAsyncData(getProducts);

  return (
    <Section>
      <SectionHeading
        eyebrow={t("merch.eyebrow")}
        title={t("merch.title")}
        subtitle={t("merch.subtitle")}
      />

      <div className="mt-12">
        {loading && <LoadingGrid count={6} />}
        {error && <ErrorState onRetry={reload} />}
        {data &&
          (data.length > 0 ? (
            <div className="grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-3">
              {data.map((product) => (
                <ProductCard key={product.id} product={product} />
              ))}
            </div>
          ) : (
            <EmptyState message={t("merch.empty")} />
          ))}
      </div>
    </Section>
  );
}
