// WEBSITE — TABLED FOR LATER. Public marketing site, not part of the active POS workflow.
import { Link } from "@tanstack/react-router";
import { useState, type ReactNode } from "react";
import { Menu, X, Phone, MessageCircle, MapPin, Mail } from "lucide-react";

import { Button } from "@/components/ui/button";
import { BUSINESS } from "@/lib/business";

export { BUSINESS };


const NAV = [
  { to: "/website", label: "Home" },
  { to: "/website/catalog", label: "Catalog" },
  { to: "/website/about", label: "About" },
  { to: "/website/trade", label: "Bulk & Trade" },
  { to: "/website/contact", label: "Contact" },
] as const;

function Wordmark({ tone = "dark" }: { tone?: "dark" | "light" }) {
  return (
    <Link to="/website" className="flex items-baseline gap-2">
      <span
        className={`font-display text-2xl leading-none ${tone === "light" ? "text-ivory" : "text-foreground"}`}
      >
        City
      </span>
      <span className="eyebrow text-brass">Tiles</span>
    </Link>
  );
}

export function SiteShell({ children }: { children: ReactNode }) {
  const [open, setOpen] = useState(false);

  return (
    <div className="min-h-screen bg-background">
      <div className="bg-marble-black text-ivory/70">
        <div className="mx-auto flex max-w-7xl flex-wrap items-center justify-between gap-2 px-5 py-2 text-xs">
          <span className="tracking-wide">Factory-direct supply since 2015 · Abbottabad</span>
          <div className="flex items-center gap-5">
            <a href={BUSINESS.phoneHref} className="flex items-center gap-1.5 hover:text-brass">
              <Phone className="size-3" /> {BUSINESS.phone}
            </a>
            <a
              href={BUSINESS.whatsapp}
              target="_blank"
              rel="noreferrer"
              className="hidden items-center gap-1.5 hover:text-brass sm:flex"
            >
              <MessageCircle className="size-3" /> WhatsApp
            </a>
          </div>
        </div>
      </div>

      <header className="sticky top-0 z-40 border-b border-border/70 bg-background/90 backdrop-blur">
        <div className="mx-auto flex max-w-7xl items-center justify-between px-5 py-4">
          <Wordmark />
          <nav className="hidden items-center gap-8 md:flex">
            {NAV.map((item) => (
              <Link
                key={item.to}
                to={item.to}
                activeOptions={{ exact: item.to === "/website" }}
                activeProps={{ className: "text-foreground" }}
                inactiveProps={{ className: "text-muted-foreground" }}
                className="text-sm tracking-wide transition-colors hover:text-foreground"
              >
                {item.label}
              </Link>
            ))}
          </nav>
          <div className="hidden md:block">
            <Button asChild variant="brass" size="lg">
              <Link to="/website/trade">Request a Quote</Link>
            </Button>
          </div>
          <button
            className="md:hidden"
            aria-label="Toggle menu"
            onClick={() => setOpen((v) => !v)}
          >
            {open ? <X className="size-5" /> : <Menu className="size-5" />}
          </button>
        </div>
        {open && (
          <nav className="border-t border-border bg-background px-5 py-4 md:hidden">
            {NAV.map((item) => (
              <Link
                key={item.to}
                to={item.to}
                onClick={() => setOpen(false)}
                className="block py-2 text-sm text-muted-foreground"
              >
                {item.label}
              </Link>
            ))}
            <Button asChild variant="brass" className="mt-3 w-full">
              <Link to="/website/trade" onClick={() => setOpen(false)}>
                Request a Quote
              </Link>
            </Button>
          </nav>
        )}
      </header>

      <main>{children}</main>

      <footer className="mt-24 bg-marble-black text-ivory/70">
        <div className="mx-auto grid max-w-7xl gap-10 px-5 py-16 md:grid-cols-4">
          <div className="md:col-span-2">
            <Wordmark tone="light" />
            <p className="mt-4 max-w-sm text-sm leading-relaxed">
              Quarry-sourced marble, tiles, terrazzo chips and sanitary ware, cut and finished at our
              own factory in Abbottabad and supplied to homeowners, architects and contractors.
            </p>
          </div>
          <div>
            <h4 className="eyebrow text-brass">Explore</h4>
            <ul className="mt-4 space-y-2 text-sm">
              {NAV.map((item) => (
                <li key={item.to}>
                  <Link to={item.to} className="hover:text-ivory">
                    {item.label}
                  </Link>
                </li>
              ))}
              <li>
                <Link to="/" className="hover:text-ivory">
                  Staff Login
                </Link>
              </li>
            </ul>
          </div>
          <div>
            <h4 className="eyebrow text-brass">Visit &amp; Call</h4>
            <ul className="mt-4 space-y-3 text-sm">
              <li className="flex gap-2">
                <MapPin className="mt-0.5 size-4 shrink-0 text-brass" /> {BUSINESS.address}
              </li>
              <li className="flex gap-2">
                <Phone className="mt-0.5 size-4 shrink-0 text-brass" />
                <a href={BUSINESS.phoneHref}>{BUSINESS.phone}</a>
              </li>
              <li className="flex gap-2">
                <Mail className="mt-0.5 size-4 shrink-0 text-brass" />
                <a href={`mailto:${BUSINESS.email}`}>{BUSINESS.email}</a>
              </li>
            </ul>
          </div>
        </div>
        <div className="border-t border-ivory/10">
          <div className="mx-auto max-w-7xl px-5 py-5 text-xs">
            © {new Date().getFullYear()} {BUSINESS.name}. All rights reserved.
          </div>
        </div>
      </footer>
    </div>
  );
}

export function PageHeader({
  eyebrow,
  title,
  intro,
}: {
  eyebrow: string;
  title: string;
  intro?: string;
}) {
  return (
    <section className="stone-surface border-b border-border">
      <div className="mx-auto max-w-7xl px-5 py-16 md:py-24">
        <p className="eyebrow text-brass">{eyebrow}</p>
        <h1 className="mt-4 max-w-3xl text-4xl md:text-6xl">{title}</h1>
        {intro && <p className="mt-5 max-w-2xl text-muted-foreground md:text-lg">{intro}</p>}
      </div>
    </section>
  );
}
