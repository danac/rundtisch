import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";

import { AsyncImage } from "../components/AsyncImage";
import { Section } from "../components/Section";

export function NotFound() {
  const { t } = useTranslation();
  return (
    <Section className="text-center">
      <div className="mx-auto max-w-md">
        <div className="mx-auto mb-8 w-48">
          <AsyncImage
            src="/images/bird-hummingbird.png"
            alt=""
            ratio="3 / 2"
            priority
            className="rounded-blob shadow-soft"
          />
        </div>
        <h1 className="text-5xl font-bold text-coral-500">404</h1>
        <p className="mt-4 text-lg text-ink-soft">
          This little bird flew off course. Let's get you back home.
        </p>
        <Link to="/" className="btn-primary mt-8">
          {t("nav.home")}
        </Link>
      </div>
    </Section>
  );
}
