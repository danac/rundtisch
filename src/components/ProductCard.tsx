import { useTranslation } from "react-i18next";
import type { Product } from "../types/merch";
import { AsyncImage } from "./AsyncImage";

const currencySymbols: Record<Product["currency"], string> = {
  USD: "$",
  EUR: "\u20ac",
  GBP: "\u00a3",
};

function formatPrice(product: Product) {
  return `${currencySymbols[product.currency]}${product.price.toFixed(2)}`;
}

export function ProductCard({ product }: { product: Product }) {
  const { t } = useTranslation();

  return (
    <article className="group flex flex-col overflow-hidden rounded-blob bg-white shadow-soft transition-all duration-300 hover:-translate-y-1 hover:shadow-lift">
      <div className="relative">
        <AsyncImage
          src={product.image}
          alt={product.alt}
          ratio="1 / 1"
          className="[&_img]:transition-transform [&_img]:duration-500 group-hover:[&_img]:scale-105"
        />
        {product.soldOut && (
          <span className="pill absolute left-4 top-4 bg-ink/80 text-white">
            {t("merch.soldOut")}
          </span>
        )}
      </div>
      <div className="flex flex-1 flex-col p-5">
        <h3 className="text-lg font-bold leading-tight">{product.title}</h3>
        <p className="mt-1 flex-1 text-sm text-ink-soft">{product.description}</p>
        <div className="mt-4 flex items-center justify-between gap-3">
          <span className="text-xl font-bold text-coral-600">
            {formatPrice(product)}
          </span>
          <a
            href={product.url}
            target="_blank"
            rel="noreferrer noopener"
            aria-disabled={product.soldOut}
            className={`btn px-5 py-2 text-sm ${
              product.soldOut
                ? "pointer-events-none bg-ink/10 text-ink-soft"
                : "bg-teal-500 text-white hover:bg-teal-600"
            }`}
          >
            {product.soldOut ? t("merch.soldOut") : t("merch.buy")}
          </a>
        </div>
      </div>
    </article>
  );
}
