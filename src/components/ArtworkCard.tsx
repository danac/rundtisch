import { useTranslation } from "react-i18next";
import type { Artwork } from "../types/portfolio";
import { AsyncImage } from "./AsyncImage";

interface ArtworkCardProps {
  artwork: Artwork;
  onSelect?: (artwork: Artwork) => void;
}

export function ArtworkCard({ artwork, onSelect }: ArtworkCardProps) {
  const { t } = useTranslation();

  return (
    <button
      type="button"
      onClick={() => onSelect?.(artwork)}
      className="group block w-full overflow-hidden rounded-blob bg-white text-left shadow-soft transition-all duration-300 hover:-translate-y-1 hover:shadow-lift focus:outline-none focus-visible:ring-4 focus-visible:ring-coral-300/50"
    >
      <AsyncImage
        src={artwork.image}
        alt={artwork.alt}
        ratio="4 / 3"
        className="[&_img]:transition-transform [&_img]:duration-500 group-hover:[&_img]:scale-105"
      />
      <div className="flex items-center justify-between gap-3 px-5 py-4">
        <div>
          <h3 className="text-lg font-bold leading-tight">{artwork.title}</h3>
          {artwork.year && (
            <p className="text-sm text-ink-soft">{artwork.year}</p>
          )}
        </div>
        <span className="pill bg-teal-50 text-teal-600">
          {t(`categories.${artwork.category}`)}
        </span>
      </div>
    </button>
  );
}
