import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";
import type { Collection } from "../types/portfolio";
import { AsyncImage } from "./AsyncImage";

interface CollectionCardProps {
  collection: Collection;
}

/** Up to 3 preview images for the mosaic; extras shown as +N overlay. */
const PREVIEW_COUNT = 3;

function CollectionMosaic({ collection }: { collection: Collection }) {
  const previews = collection.artworks.slice(0, PREVIEW_COUNT);
  const extra = collection.artworks.length - PREVIEW_COUNT;
  const count = previews.length;

  if (count === 0) {
    return (
      <div className="aspect-[4/3] bg-paper" aria-hidden />
    );
  }

  if (count === 1) {
    return (
      <AsyncImage
        src={previews[0].image}
        alt=""
        ratio="4 / 3"
        className="[&_img]:transition-transform [&_img]:duration-500 group-hover:[&_img]:scale-105"
      />
    );
  }

  if (count === 2) {
    return (
      <div className="grid aspect-[4/3] grid-cols-2 gap-0.5">
        {previews.map((art) => (
          <AsyncImage
            key={art.id}
            src={art.image}
            alt=""
            ratio="1 / 1"
            className="h-full [&_img]:transition-transform [&_img]:duration-500 group-hover:[&_img]:scale-105"
          />
        ))}
      </div>
    );
  }

  // 3+ previews: large left tile + stacked right column
  return (
    <div className="grid aspect-[4/3] grid-cols-3 gap-0.5">
      <div className="col-span-2 row-span-2">
        <AsyncImage
          src={previews[0].image}
          alt=""
          ratio="1 / 1"
          className="h-full [&_img]:transition-transform [&_img]:duration-500 group-hover:[&_img]:scale-105"
        />
      </div>
      <AsyncImage
        src={previews[1].image}
        alt=""
        ratio="1 / 1"
        className="[&_img]:transition-transform [&_img]:duration-500 group-hover:[&_img]:scale-105"
      />
      <div className="relative">
        <AsyncImage
          src={previews[2].image}
          alt=""
          ratio="1 / 1"
          className="[&_img]:transition-transform [&_img]:duration-500 group-hover:[&_img]:scale-105"
        />
        {extra > 0 && (
          <span
            aria-hidden
            className="absolute inset-0 flex items-center justify-center bg-ink/45 text-lg font-bold text-white"
          >
            +{extra}
          </span>
        )}
      </div>
    </div>
  );
}

export function CollectionCard({ collection }: CollectionCardProps) {
  const { t } = useTranslation();
  const pieceCount = collection.artworks.length;

  return (
    <Link
      to={`/portfolio/${collection.slug}`}
      className="group block overflow-hidden rounded-blob bg-white shadow-soft transition-all duration-300 hover:-translate-y-1 hover:shadow-lift focus:outline-none focus-visible:ring-4 focus-visible:ring-coral-300/50"
    >
      <div className="overflow-hidden">
        <CollectionMosaic collection={collection} />
      </div>
      <div className="flex items-start justify-between gap-3 px-5 py-4">
        <div>
          <h3 className="text-lg font-bold leading-tight">{collection.title}</h3>
          <p className="mt-1 text-sm text-ink-soft">
            {t("portfolio.pieceCount", { count: pieceCount })}
          </p>
        </div>
        <span className="pill shrink-0 bg-teal-50 text-teal-600">
          {t(`categories.${collection.category}`)}
        </span>
      </div>
    </Link>
  );
}
