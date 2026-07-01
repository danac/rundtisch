import { useEffect, useState } from "react";
import { Link, NavLink, useLocation } from "react-router-dom";
import { useTranslation } from "react-i18next";

interface NavItem {
  to: string;
  key: string;
  end?: boolean;
}

const navItems: NavItem[] = [
  { to: "/", key: "nav.home", end: true },
  { to: "/portfolio", key: "nav.portfolio" },
  { to: "/shop", key: "nav.merch" },
  { to: "/about", key: "nav.about" },
];

function Logo() {
  const { t } = useTranslation();
  return (
    <Link to="/" className="group flex flex-col leading-none">
      <span className="font-script text-3xl font-bold text-coral-500 transition-colors group-hover:text-coral-600 sm:text-4xl">
        {t("brand.name")}
      </span>
      <span className="text-[0.7rem] font-semibold uppercase tracking-[0.16em] text-ink-soft">
        {t("brand.tagline")}
      </span>
    </Link>
  );
}

export function Navbar() {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const location = useLocation();

  // Close the mobile menu on navigation.
  useEffect(() => {
    setOpen(false);
  }, [location.pathname]);

  // Prevent background scroll while the mobile menu is open.
  useEffect(() => {
    document.body.style.overflow = open ? "hidden" : "";
    return () => {
      document.body.style.overflow = "";
    };
  }, [open]);

  const linkClass = ({ isActive }: { isActive: boolean }) =>
    `relative px-1 py-2 text-base font-semibold transition-colors ${
      isActive ? "text-coral-600" : "text-ink hover:text-coral-500"
    } after:absolute after:-bottom-0.5 after:left-0 after:h-0.5 after:rounded-full after:bg-coral-500 after:transition-all after:duration-300 ${
      isActive ? "after:w-full" : "after:w-0 hover:after:w-full"
    }`;

  return (
    <header className="sticky top-0 z-40 border-b border-ink/5 bg-cream/85 backdrop-blur-md">
      <nav className="container-page flex items-center justify-between gap-4 py-3">
        <Logo />

        <div className="hidden items-center gap-7 md:flex">
          {navItems.map((item) => (
            <NavLink key={item.to} to={item.to} end={item.end} className={linkClass}>
              {t(item.key)}
            </NavLink>
          ))}
        </div>

        <button
          type="button"
          onClick={() => setOpen((v) => !v)}
          aria-expanded={open}
          aria-label={open ? t("nav.closeMenu") : t("nav.openMenu")}
          className="flex h-11 w-11 items-center justify-center rounded-full border-2 border-ink/10 bg-white/70 text-ink transition-colors hover:border-coral-300 md:hidden"
        >
          <span className="relative block h-4 w-5">
            <span
              className={`absolute left-0 block h-0.5 w-5 rounded-full bg-current transition-all duration-300 ${
                open ? "top-1.5 rotate-45" : "top-0"
              }`}
            />
            <span
              className={`absolute left-0 top-1.5 block h-0.5 w-5 rounded-full bg-current transition-all duration-300 ${
                open ? "opacity-0" : "opacity-100"
              }`}
            />
            <span
              className={`absolute left-0 block h-0.5 w-5 rounded-full bg-current transition-all duration-300 ${
                open ? "top-1.5 -rotate-45" : "top-3"
              }`}
            />
          </span>
        </button>
      </nav>

      {/* Mobile menu */}
      <div
        className={`overflow-hidden border-t border-ink/5 bg-cream md:hidden ${
          open ? "max-h-96" : "max-h-0"
        } transition-[max-height] duration-300 ease-out`}
      >
        <div className="container-page flex flex-col gap-1 py-4">
          {navItems.map((item) => (
            <NavLink
              key={item.to}
              to={item.to}
              end={item.end}
              className={({ isActive }) =>
                `rounded-xl px-4 py-3 text-lg font-semibold transition-colors ${
                  isActive
                    ? "bg-coral-50 text-coral-600"
                    : "text-ink hover:bg-paper"
                }`
              }
            >
              {t(item.key)}
            </NavLink>
          ))}
        </div>
      </div>
    </header>
  );
}
