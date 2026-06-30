import { useCallback } from "react";
import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";

import { AsyncImage } from "../components/AsyncImage";
import { CollectionCard } from "../components/CollectionCard";
import { Section, SectionHeading } from "../components/Section";
import { LoadingGrid, ErrorState } from "../components/States";
import { useAsyncData } from "../hooks/useAsyncData";
import { getFeaturedCollections } from "../services/portfolioService";

function Hero() {
  const { t } = useTranslation();
  return (
    <section className="relative overflow-hidden">
      {/* Soft watercolor blobs */}
      <div
        aria-hidden
        className="pointer-events-none absolute -left-24 -top-24 h-72 w-72 rounded-full bg-teal-100 blur-3xl"
      />
      <div
        aria-hidden
        className="pointer-events-none absolute -right-16 top-32 h-80 w-80 rounded-full bg-coral-100 blur-3xl"
      />
      <div className="container-page relative grid items-center gap-10 py-8 sm:py-10 lg:grid-cols-2 lg:gap-12">
        <div className="animate-fade-up">
          <p className="mb-4 text-sm font-bold uppercase tracking-[0.18em] text-coral-500">
            {t("home.hero.eyebrow")}
          </p>
          <h1 className="text-4xl font-bold leading-[1.05] sm:text-5xl lg:text-6xl">
            {t("home.hero.title")}
          </h1>
          <p className="mt-6 max-w-xl text-lg text-ink-soft sm:text-xl">
            {t("home.hero.subtitle")}
          </p>
          <div className="mt-8 flex flex-wrap gap-3">
            <Link to="/portfolio" className="btn-primary">
              {t("home.hero.ctaPortfolio")}
            </Link>
            <Link to="/shop" className="btn-ghost">
              {t("home.hero.ctaShop")}
            </Link>
          </div>
        </div>

        <div className="relative animate-fade-in">
          <div className="absolute inset-0 -rotate-3 rounded-blob bg-sun-300/30" />
          <AsyncImage
            src="/images/bird-bluetits.png"
            alt={t("home.hero.title")}
            ratio="3 / 2"
            priority
            className="relative rounded-blob shadow-lift"
          />
          <div className="absolute -bottom-6 -left-6 hidden h-28 w-28 animate-float overflow-hidden rounded-3xl shadow-lift sm:block">
            <AsyncImage
              src="/images/bird-hummingbird.png"
              alt=""
              ratio="1 / 1"
              priority
            />
          </div>
        </div>
      </div>
    </section>
  );
}

function Categories() {
  const { t } = useTranslation();
  const cards = [
    {
      key: "birds",
      image: "/images/bird-robin.png",
      to: "/portfolio",
      accent: "bg-coral-50 text-coral-600",
    },
    {
      key: "lettering",
      image: "/images/lettering-hello.png",
      to: "/portfolio",
      accent: "bg-teal-50 text-teal-600",
    },
    {
      key: "children",
      image: "/images/kids-teaparty.png",
      to: "/portfolio",
      accent: "bg-sun-300/30 text-ink",
    },
  ] as const;

  return (
    <Section className="bg-paper !py-4 sm:!py-5">
      <SectionHeading
        eyebrow={t("home.categories.eyebrow")}
        title={t("home.categories.title")}
      />
      <div className="mt-12 grid gap-6 sm:grid-cols-3">
        {cards.map((card) => (
          <Link
            key={card.key}
            to={card.to}
            className="group overflow-hidden rounded-blob bg-white shadow-soft transition-all duration-300 hover:-translate-y-1 hover:shadow-lift"
          >
            <AsyncImage
              src={card.image}
              alt={t(`home.categories.${card.key}.title`)}
              ratio="4 / 3"
              className="[&_img]:transition-transform [&_img]:duration-500 group-hover:[&_img]:scale-105"
            />
            <div className="p-6">
              <span className={`pill ${card.accent}`}>
                {t(`categories.${card.key}`)}
              </span>
              <h3 className="mt-3 text-xl font-bold">
                {t(`home.categories.${card.key}.title`)}
              </h3>
              <p className="mt-2 text-ink-soft">
                {t(`home.categories.${card.key}.text`)}
              </p>
            </div>
          </Link>
        ))}
      </div>
    </Section>
  );
}

function FeaturedWork() {
  const { t } = useTranslation();
  const fetcher = useCallback(
    (signal: AbortSignal) => getFeaturedCollections(3, signal),
    [],
  );
  const { data, loading, error, reload } = useAsyncData(fetcher);

  return (
    <Section className="!py-4 sm:!py-5">
      <SectionHeading
        eyebrow={t("home.featured.eyebrow")}
        title={t("home.featured.title")}
        subtitle={t("home.featured.subtitle")}
      />
      <div className="mt-12">
        {loading && <LoadingGrid count={3} />}
        {error && <ErrorState onRetry={reload} />}
        {data && (
          <div className="grid grid-cols-1 gap-5 sm:grid-cols-2 lg:grid-cols-3">
            {data.map((collection) => (
              <CollectionCard key={collection.id} collection={collection} />
            ))}
          </div>
        )}
      </div>
      <div className="mt-10 text-center">
        <Link to="/portfolio" className="btn-secondary">
          {t("home.featured.viewAll")}
        </Link>
      </div>
    </Section>
  );
}

function CallToAction() {
  const { t } = useTranslation();
  return (
    <Section className="!py-4 sm:!py-5">
      <div className="relative overflow-hidden rounded-blob bg-gradient-to-br from-coral-500 to-blossom-500 px-8 py-14 text-center text-white shadow-lift sm:px-16">
        <h2 className="text-3xl font-bold text-white sm:text-4xl">
          {t("home.cta.title")}
        </h2>
        <p className="mx-auto mt-4 max-w-xl text-lg text-white/90">
          {t("home.cta.subtitle")}
        </p>
        <a
          href="mailto:hello@example.com"
          className="btn mt-8 bg-white text-coral-600 hover:-translate-y-0.5 hover:bg-cream"
        >
          {t("home.cta.button")}
        </a>
      </div>
    </Section>
  );
}

export function Home() {
  return (
    <>
      <Hero />
      <FeaturedWork />
      <Categories />
      <CallToAction />
    </>
  );
}
