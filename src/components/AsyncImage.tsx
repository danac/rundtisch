import { useEffect, useRef, useState } from "react";

interface AsyncImageProps {
  src: string;
  alt: string;
  className?: string;
  /** Aspect ratio applied to the wrapper, e.g. "4 / 3". */
  ratio?: string;
  /** Render eagerly (e.g. for above-the-fold hero images). */
  priority?: boolean;
  sizes?: string;
}

/**
 * Image that loads asynchronously and smoothly:
 * - only starts loading when near the viewport (IntersectionObserver)
 * - native lazy + async decoding as a fallback
 * - shows a soft shimmer placeholder, then fades the image in on load
 */
export function AsyncImage({
  src,
  alt,
  className = "",
  ratio = "4 / 3",
  priority = false,
  sizes,
}: AsyncImageProps) {
  const wrapperRef = useRef<HTMLDivElement | null>(null);
  const [inView, setInView] = useState(priority);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    if (priority || inView) return;
    const el = wrapperRef.current;
    if (!el) return;

    if (typeof IntersectionObserver === "undefined") {
      setInView(true);
      return;
    }

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting)) {
          setInView(true);
          observer.disconnect();
        }
      },
      { rootMargin: "200px" },
    );

    observer.observe(el);
    return () => observer.disconnect();
  }, [priority, inView]);

  return (
    <div
      ref={wrapperRef}
      className={`relative overflow-hidden bg-paper ${className}`}
      style={{ aspectRatio: ratio }}
    >
      <div
        aria-hidden
        className={`absolute inset-0 bg-gradient-to-br from-teal-50 via-coral-50 to-sun-300/20 transition-opacity duration-500 ${
          loaded ? "opacity-0" : "animate-pulse opacity-100"
        }`}
      />
      {inView && (
        <img
          src={src}
          alt={alt}
          loading={priority ? "eager" : "lazy"}
          decoding="async"
          sizes={sizes}
          onLoad={() => setLoaded(true)}
          className={`relative h-full w-full object-cover transition-all duration-700 ease-out ${
            loaded ? "scale-100 opacity-100 blur-0" : "scale-105 opacity-0 blur-md"
          }`}
        />
      )}
    </div>
  );
}
