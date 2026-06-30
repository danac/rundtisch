import { useState } from "react";
import type { Artwork } from "../types/portfolio";
import { ArtworkCard } from "./ArtworkCard";
import { Lightbox } from "./Lightbox";

export function Gallery({ items }: { items: Artwork[] }) {
  const [activeIndex, setActiveIndex] = useState<number | null>(null);

  return (
    <>
      <div className="grid grid-cols-1 gap-5 sm:grid-cols-2 lg:grid-cols-3">
        {items.map((artwork, i) => (
          <ArtworkCard
            key={artwork.id}
            artwork={artwork}
            onSelect={() => setActiveIndex(i)}
          />
        ))}
      </div>

      {activeIndex !== null && (
        <Lightbox
          items={items}
          index={activeIndex}
          onClose={() => setActiveIndex(null)}
          onNavigate={setActiveIndex}
        />
      )}
    </>
  );
}
