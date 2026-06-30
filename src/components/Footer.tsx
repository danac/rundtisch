import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";

const socials = [
  { label: "Instagram", href: "https://instagram.com" },
  { label: "Etsy", href: "https://etsy.com" },
  { label: "Patreon", href: "https://patreon.com" },
];

export function Footer() {
  const { t } = useTranslation();
  const year = new Date().getFullYear();

  const exploreLinks = [
    { to: "/portfolio", key: "nav.portfolio" },
    { to: "/shop", key: "nav.merch" },
    { to: "/about", key: "nav.about" },
  ];

  return (
    <footer className="mt-8 border-t border-ink/5 bg-paper">
      <div className="container-page grid gap-10 py-14 sm:grid-cols-2 lg:grid-cols-4">
        <div className="sm:col-span-2 lg:col-span-1">
          <span className="font-script text-3xl font-bold text-coral-500">
            {t("brand.name")}
          </span>
          <p className="mt-3 max-w-xs text-ink-soft">{t("footer.tagline")}</p>
        </div>

        <div>
          <h3 className="text-sm font-bold uppercase tracking-[0.16em] text-ink-soft">
            {t("footer.explore")}
          </h3>
          <ul className="mt-4 space-y-2">
            {exploreLinks.map((link) => (
              <li key={link.to}>
                <Link
                  to={link.to}
                  className="font-semibold text-ink transition-colors hover:text-coral-500"
                >
                  {t(link.key)}
                </Link>
              </li>
            ))}
          </ul>
        </div>

        <div>
          <h3 className="text-sm font-bold uppercase tracking-[0.16em] text-ink-soft">
            {t("footer.connect")}
          </h3>
          <ul className="mt-4 space-y-2">
            {socials.map((s) => (
              <li key={s.label}>
                <a
                  href={s.href}
                  target="_blank"
                  rel="noreferrer noopener"
                  className="font-semibold text-ink transition-colors hover:text-coral-500"
                >
                  {s.label}
                </a>
              </li>
            ))}
          </ul>
        </div>

        <div>
          <h3 className="text-sm font-bold uppercase tracking-[0.16em] text-ink-soft">
            {t("footer.newsletter")}
          </h3>
          <p className="mt-4 text-sm text-ink-soft">{t("footer.newsletterText")}</p>
          <form
            className="mt-4 flex flex-col gap-2 sm:flex-row"
            onSubmit={(e) => e.preventDefault()}
          >
            <input
              type="email"
              required
              placeholder={t("footer.emailPlaceholder")}
              className="w-full rounded-full border-2 border-ink/10 bg-white px-4 py-2 text-sm focus:border-coral-300 focus:outline-none"
            />
            <button type="submit" className="btn-primary px-5 py-2 text-sm">
              {t("footer.subscribe")}
            </button>
          </form>
        </div>
      </div>

      <div className="border-t border-ink/5">
        <div className="container-page flex flex-col items-center justify-between gap-2 py-6 text-sm text-ink-soft sm:flex-row">
          <p>
            &copy; {year} {t("brand.name")}. {t("footer.rights")}
          </p>
          <p>{t("footer.madeWith")}</p>
        </div>
      </div>
    </footer>
  );
}
