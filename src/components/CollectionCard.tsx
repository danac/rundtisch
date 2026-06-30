import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";
import type { Collection } from "../types/portfolio";
import { AsyncImage } from "./AsyncImage";

interface CollectionCardProps {
  collection: Collection;
}

/** Visual config per stack position: 0 = front, 1 = middle, 2 = back. */
const STACK_LAYERS = [
  {
    rotate: "rotate-0",
    hoverRotate: "group-hover:rotate-[1.5deg]",
    translate: "translate-x-0 translate-y-0",
    hoverTranslate: "group-hover:translate-x-1 group-hover:translate-y-1",
    scale: "scale-100",
    opacity: "opacity-100",
    z: "z-30",
    shadow:
      "shadow-[0_10px_28px_-10px_rgba(47,42,61,0.28)] group-hover:shadow-[0_14px_32px_-10px_rgba(47,42,61,0.32)]",
  },
  {
    rotate: "rotate-[-4deg]",
    hoverRotate: "group-hover:rotate-[-7deg]",
    translate: "-translate-x-2 -translate-y-3",
    hoverTranslate: "group-hover:-translate-x-5 group-hover:-translate-y-6",
    scale: "scale-[0.94]",
    opacity: "opacity-100",
    z: "z-20",
    shadow:
      "shadow-[0_8px_22px_-8px_rgba(47,42,61,0.22)] group-hover:shadow-[0_12px_26px_-8px_rgba(47,42,61,0.26)]",
  },
  {
    rotate: "rotate-[-8deg]",
    hoverRotate: "group-hover:rotate-[-12deg]",
    translate: "-translate-x-4 -translate-y-6",
    hoverTranslate: "group-hover:-translate-x-8 group-hover:-translate-y-10",
    scale: "scale-[0.88]",
    opacity: "opacity-100",
    z: "z-10",
    shadow:
      "shadow-[0_6px_18px_-6px_rgba(47,42,61,0.18)] group-hover:shadow-[0_10px_22px_-6px_rgba(47,42,61,0.22)]",
  },
] as const;

function CollectionStack({ collection }: { collection: Collection }) {
  const previews = collection.artworks.slice(0, 3);

  if (previews.length === 0) {
    return <div className="aspect-[4/3] bg-paper" aria-hidden />;
  }

  if (previews.length === 1) {
    return (
      <div className="aspect-[4/3] overflow-hidden bg-paper px-5 py-6">
        <div className="h-full overflow-hidden rounded-soft shadow-[0_8px_22px_-8px_rgba(47,42,61,0.22)] ring-2 ring-white/80 transition-shadow duration-300 group-hover:shadow-[0_12px_28px_-8px_rgba(47,42,61,0.28)]">
          <AsyncImage
            src={previews[0].image}
            alt=""
            ratio="4 / 3"
            className="h-full [&_img]:object-cover [&_img]:transition-transform [&_img]:duration-500 group-hover:[&_img]:scale-105"
          />
        </div>
      </div>
    );
  }

  const stacked = previews.map((art, i) => ({
    art,
    layer: STACK_LAYERS[i],
  }));

  return (
    <div className="relative aspect-[4/3] overflow-hidden bg-paper px-5 py-6">
      {[...stacked].reverse().map(({ art, layer }) => {
        const isFront = layer.z === "z-30";
        return (
          <div
            key={art.id}
            aria-hidden={!isFront}
            className={[
              "pointer-events-none absolute inset-x-5 top-6 bottom-4 origin-center",
              "transition-all duration-300 ease-out",
              layer.z,
              layer.rotate,
              layer.hoverRotate,
              layer.translate,
              layer.hoverTranslate,
              layer.scale,
              layer.opacity,
            ].join(" ")}
          >
            <div
              className={`h-full overflow-hidden rounded-soft ring-2 ring-white/80 transition-shadow duration-300 ${layer.shadow}`}
            >
              <AsyncImage
                src={art.image}
                alt=""
                ratio="4 / 3"
                className="h-full [&_img]:object-cover [&_img]:transition-transform [&_img]:duration-500 group-hover:[&_img]:scale-[1.02]"
              />
            </div>
          </div>
        );
      })}
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
      <CollectionStack collection={collection} />
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
