import { createFileRoute, Link } from "@tanstack/react-router";
import { ShieldCheck, Truck, Hammer, Ruler } from "lucide-react";

import { Button } from "@/components/ui/button";
import { SiteShell, BUSINESS } from "@/components/website/SiteShell";
import { CATEGORIES } from "@/lib/catalog";
const heroImg = "/images/hero-marble.jpg";
const factoryImg = "/images/factory.jpg";

export const Route = createFileRoute("/website/")({
  head: () => ({
    meta: [
      { title: "City Tiles — Marble, Tiles, Chips & Sanitary" },
      {
        name: "description",
        content:
          "Factory-direct natural marble, porcelain tiles, terrazzo flooring chips and sanitary ware for homes, architects and contractors in Abbottabad.",
      },
      { property: "og:title", content: "City Tiles" },
      {
        property: "og:description",
        content:
          "Factory-direct natural marble, tiles, terrazzo chips and sanitary ware — retail and bulk supply.",
      },
    ],
  }),
  component: Index,
});

const TRUST = [
  { icon: Hammer, title: "Own cutting factory", text: "Slabs cut, honed and polished in-house." },
  { icon: ShieldCheck, title: "Graded quality", text: "Every batch checked for tone and thickness." },
  { icon: Truck, title: "Bulk supply", text: "Site delivery across AJK and Punjab." },
  { icon: Ruler, title: "Site measurement", text: "Free measurement and layout guidance." },
];

function Index() {
  return (
    <SiteShell>
      <section className="relative isolate overflow-hidden bg-marble-black">
        <img
          src={heroImg}
          alt="Polished natural marble slab with warm ivory tones and charcoal veining"
          width={1920}
          height={1088}
          className="absolute inset-0 size-full object-cover opacity-70"
        />
        <div className="absolute inset-0 bg-gradient-to-r from-marble-black via-marble-black/70 to-transparent" />
        <div className="relative mx-auto grid max-w-7xl px-5 py-28 md:py-40">
          <div className="max-w-2xl">
            <p className="eyebrow text-brass">Abbottabad, Khyber Pakhtunkhwa · Since 2015</p>
            <h1 className="mt-6 text-5xl leading-[1.05] text-ivory md:text-7xl">
              Natural stone, cut and finished at our own factory.
            </h1>
            <p className="mt-6 max-w-xl text-lg text-ivory/75">
              Marble slabs, porcelain tiles, terrazzo flooring chips and sanitary ware — supplied
              piece by piece to homeowners and by the truckload to contractors.
            </p>
            <div className="mt-10 flex flex-wrap gap-4">
              <Button asChild variant="brass" size="xl">
                <Link to="/website/catalog">Browse the Catalog</Link>
              </Button>
              <Button asChild variant="onDark" size="xl">
                <Link to="/website/trade">Bulk Pricing</Link>
              </Button>
            </div>
          </div>
        </div>
      </section>

      <section className="border-b border-border bg-card">
        <div className="mx-auto grid max-w-7xl gap-8 px-5 py-12 sm:grid-cols-2 lg:grid-cols-4">
          {TRUST.map((item) => (
            <div key={item.title} className="flex gap-4">
              <item.icon className="size-5 shrink-0 text-brass" />
              <div>
                <h3 className="text-lg">{item.title}</h3>
                <p className="mt-1 text-sm text-muted-foreground">{item.text}</p>
              </div>
            </div>
          ))}
        </div>
      </section>

      <section className="mx-auto max-w-7xl px-5 py-24">
        <div className="flex flex-wrap items-end justify-between gap-6">
          <div>
            <p className="eyebrow text-brass">What we supply</p>
            <h2 className="mt-3 text-4xl md:text-5xl">Four material families</h2>
          </div>
          <Link to="/website/catalog" className="eyebrow text-foreground/70 hover:text-brass">
            View full catalog →
          </Link>
        </div>
        <div className="mt-12 grid gap-6 sm:grid-cols-2 lg:grid-cols-4">
          {CATEGORIES.map((cat) => (
            <Link
              key={cat.key}
              to="/website/catalog"
              search={{ category: cat.key }}
              className="group relative isolate overflow-hidden border border-border"
            >
              <img
                src={cat.image}
                alt={cat.label}
                loading="lazy"
                width={1024}
                height={1280}
                className="aspect-3/4 w-full object-cover transition-transform duration-700 group-hover:scale-105"
              />
              <div className="absolute inset-0 bg-gradient-to-t from-marble-black/90 via-marble-black/20 to-transparent" />
              <div className="absolute inset-x-0 bottom-0 p-6">
                <h3 className="text-2xl text-ivory">{cat.label}</h3>
                <p className="mt-2 text-sm text-ivory/70">{cat.blurb}</p>
              </div>
            </Link>
          ))}
        </div>
      </section>

      <section className="stone-surface border-y border-border">
        <div className="mx-auto grid max-w-7xl items-center gap-14 px-5 py-24 lg:grid-cols-2">
          <img
            src={factoryImg}
            alt="Worker handling a large white marble slab inside the cutting workshop"
            loading="lazy"
            width={1600}
            height={1024}
            className="w-full object-cover shadow-stone"
          />
          <div>
            <p className="eyebrow text-brass">The factory</p>
            <h2 className="mt-3 text-4xl md:text-5xl">Eleven years of cutting stone in Abbottabad</h2>
            <p className="mt-6 text-muted-foreground">
              We buy blocks straight from the quarries of Azad Kashmir and Balochistan, then saw,
              hone and polish them on our own lines. That means fewer hands between the mountain and
              your site — and a price that reflects it.
            </p>
            <dl className="mt-10 grid grid-cols-3 gap-6 border-t border-border pt-8">
              {[
                ["11", "Years supplying"],
                ["120k", "Sq.ft in stock"],
                ["400+", "Trade accounts"],
              ].map(([value, label]) => (
                <div key={label}>
                  <dt className="font-display text-4xl">{value}</dt>
                  <dd className="mt-1 text-xs tracking-wide text-muted-foreground uppercase">
                    {label}
                  </dd>
                </div>
              ))}
            </dl>
            <Button asChild variant="stone" size="xl" className="mt-10">
              <Link to="/website/about">Our process</Link>
            </Button>
          </div>
        </div>
      </section>

      <section className="mx-auto max-w-7xl px-5 py-24">
        <div className="bg-marble-black px-8 py-16 text-center md:px-16">
          <p className="eyebrow text-brass">Building or renovating?</p>
          <h2 className="mx-auto mt-4 max-w-2xl text-4xl text-ivory md:text-5xl">
            Send us your quantities — we'll price it the same day.
          </h2>
          <div className="mt-10 flex flex-wrap justify-center gap-4">
            <Button asChild variant="brass" size="xl">
              <Link to="/website/trade">Request bulk pricing</Link>
            </Button>
            <Button asChild variant="onDark" size="xl">
              <a href={BUSINESS.whatsapp} target="_blank" rel="noreferrer">
                WhatsApp us
              </a>
            </Button>
          </div>
        </div>
      </section>
    </SiteShell>
  );
}
