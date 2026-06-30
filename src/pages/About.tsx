import { useTranslation } from "react-i18next";

import { AsyncImage } from "../components/AsyncImage";
import { Section } from "../components/Section";

export function About() {
  const { t } = useTranslation();

  const facts = ["medium", "based", "loves", "work"] as const;

  return (
    <Section>
      <div className="grid items-start gap-10 lg:grid-cols-[5fr_6fr] lg:gap-14">
        <div className="lg:sticky lg:top-28">
          <div className="relative">
            <div
              aria-hidden
              className="absolute inset-0 -rotate-3 rounded-blob bg-teal-100"
            />
            <AsyncImage
              src="/images/bird-owl.png"
              alt={t("about.title")}
              ratio="1 / 1"
              priority
              className="relative rounded-blob shadow-lift"
            />
          </div>

          <div className="mt-8 rounded-blob bg-paper p-6">
            <h2 className="text-xl font-bold">{t("about.factsTitle")}</h2>
            <ul className="mt-4 space-y-3">
              {facts.map((fact) => (
                <li key={fact} className="flex items-start gap-3 text-ink-soft">
                  <span
                    aria-hidden
                    className="mt-2 h-2 w-2 shrink-0 rounded-full bg-coral-500"
                  />
                  <span>{t(`about.facts.${fact}`)}</span>
                </li>
              ))}
            </ul>
          </div>
        </div>

        <div className="animate-fade-up">
          <p className="mb-3 text-sm font-bold uppercase tracking-[0.18em] text-coral-500">
            {t("about.eyebrow")}
          </p>
          <h1 className="text-3xl font-bold leading-tight sm:text-4xl">
            {t("about.title")}
          </h1>
          <div className="mt-6 space-y-5 text-lg text-ink-soft">
            <p className="text-xl text-ink">{t("about.intro")}</p>
            <p>{t("about.bodyOne")}</p>
            <p>{t("about.bodyTwo")}</p>
          </div>
          <a href="mailto:hello@example.com" className="btn-primary mt-8">
            {t("about.cta")}
          </a>
        </div>
      </div>
    </Section>
  );
}
