import { createFileRoute, Link } from "@tanstack/react-router";

import { SiteShell, PageHeader } from "@/components/website/SiteShell";
import { Button } from "@/components/ui/button";
const factoryImg = "/images/factory.jpg";
const marbleImg = "/images/cat-marble.jpg";

export const Route = createFileRoute("/website/about")({
  head: () => ({
    meta: [
      { title: "About the Factory — City Tiles" },
      {
        name: "description",
        content:
          "How City Tiles sources blocks, cuts and finishes stone, and checks quality before it leaves the yard.",
      },
      { property: "og:title", content: "About City Tiles" },
      {
        property: "og:description",
        content: "Quarry sourcing, in-house cutting and finishing, and our quality standards.",
      },
    ],
  }),
  component: AboutPage,
});

const STEPS = [
  {
    step: "01",
    title: "Block selection",
    text: "Our buyers inspect blocks at the quarry face in Azad Kashmir and Balochistan, rejecting anything with hairline fracture or inconsistent tone.",
  },
  {
    step: "02",
    title: "Sawing & sizing",
    text: "Gang saws and bridge cutters take blocks down to slabs and tiles in the sizes contractors actually order.",
  },
  {
    step: "03",
    title: "Honing & polishing",
    text: "Multi-head polishing lines deliver matte, honed, brushed or mirror finishes to a consistent thickness.",
  },
  {
    step: "04",
    title: "Grading & dispatch",
    text: "Every pallet is graded, batch-matched and wrapped before loading, so a floor laid in one room matches the next.",
  },
];

function AboutPage() {
  return (
    <SiteShell>
      <PageHeader
        eyebrow="Our story"
        title="A family stone factory in the hills of Abbottabad"
        intro="What began in 2015 as a single cutting frame is now a full marble, tile, chip and sanitary supply house serving homeowners, architects and contractors."
      />

      <section className="mx-auto grid max-w-7xl items-center gap-14 px-5 py-24 lg:grid-cols-2">
        <img
          src={marbleImg}
          alt="Stacked marble slabs in the factory yard"
          loading="lazy"
          width={1024}
          height={1280}
          className="w-full object-cover shadow-stone"
        />
        <div>
          <p className="eyebrow text-brass">Why buyers stay</p>
          <h2 className="mt-3 text-4xl md:text-5xl">We own every step after the quarry</h2>
          <p className="mt-6 text-muted-foreground">
            Most suppliers buy finished material and resell it. We buy blocks. That gives us control
            over thickness, finish and batch matching — and it removes a layer of margin that would
            otherwise land on your invoice.
          </p>
          <p className="mt-4 text-muted-foreground">
            The same yard stocks porcelain and ceramic tile, coloured terrazzo chips and a full
            sanitary range, so a single delivery can finish a floor, a wall and a washroom.
          </p>
        </div>
      </section>

      <section className="stone-surface border-y border-border">
        <div className="mx-auto max-w-7xl px-5 py-24">
          <p className="eyebrow text-brass">Process</p>
          <h2 className="mt-3 text-4xl md:text-5xl">From block to pallet</h2>
          <div className="mt-14 grid gap-10 md:grid-cols-2 lg:grid-cols-4">
            {STEPS.map((s) => (
              <div key={s.step} className="border-t border-foreground/20 pt-6">
                <span className="font-display text-3xl text-brass">{s.step}</span>
                <h3 className="mt-3 text-2xl">{s.title}</h3>
                <p className="mt-3 text-sm text-muted-foreground">{s.text}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      <section className="mx-auto grid max-w-7xl items-center gap-14 px-5 py-24 lg:grid-cols-2">
        <div>
          <p className="eyebrow text-brass">Quality standards</p>
          <h2 className="mt-3 text-4xl md:text-5xl">What we guarantee</h2>
          <ul className="mt-8 space-y-4 text-muted-foreground">
            <li className="border-l-2 border-brass pl-4">
              Thickness tolerance held within ±1 mm across a batch.
            </li>
            <li className="border-l-2 border-brass pl-4">
              Tone-matched pallets, labelled by batch so repeat orders align.
            </li>
            <li className="border-l-2 border-brass pl-4">
              Replacement of any slab that arrives cracked in our own transport.
            </li>
            <li className="border-l-2 border-brass pl-4">
              Written quotations valid for 15 days on bulk orders.
            </li>
          </ul>
          <Button asChild variant="brass" size="xl" className="mt-10">
            <Link to="/website/contact">Visit the yard</Link>
          </Button>
        </div>
        <img
          src={factoryImg}
          alt="Marble slab being handled in the cutting workshop"
          loading="lazy"
          width={1600}
          height={1024}
          className="w-full object-cover shadow-stone"
        />
      </section>
    </SiteShell>
  );
}
