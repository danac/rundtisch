import type { Artwork } from "../types/portfolio";
import { AsyncImage } from "./AsyncImage";

interface ArtworkCardProps {
  artwork: Artwork;
  onSelect?: (artwork: Artwork) => void;
}

export function ArtworkCard({ artwork, onSelect }: ArtworkCardProps) {
  return (
    <button
      type="button"
      onClick={() => onSelect?.(artwork)}
      className="group block w-full overflow-hidden rounded-blob bg-white text-left shadow-soft transition-all duration-300 hover:-translate-y-1 hover:shadow-lift focus:outline-none focus-visible:ring-4 focus-visible:ring-coral-300/50"
    >
      <AsyncImage
        src={artwork.thumbnail}
        alt={artwork.alt}
        ratio="4 / 3"
        className="[&_img]:transition-transform [&_img]:duration-500 group-hover:[&_img]:scale-105"
      />
      <div className="px-5 py-4">
        <h3 className="text-lg font-bold leading-tight">{artwork.title}</h3>
        {artwork.year && (
          <p className="mt-1 text-sm text-ink-soft">{artwork.year}</p>
        )}
      </div>
    </button>
  );
}
