import { createFileRoute } from "@tanstack/react-router";

import { SiteShell, PageHeader, BUSINESS } from "@/components/website/SiteShell";
import { InquiryForm } from "@/components/website/InquiryForm";

export const Route = createFileRoute("/website/trade")({
  head: () => ({
    meta: [
      { title: "Bulk & Trade Pricing — City Tiles" },
      {
        name: "description",
        content:
          "Contractors, builders and architects: request bulk pricing on marble, tiles, terrazzo chips and sanitary ware with site delivery.",
      },
      { property: "og:title", content: "Bulk & Trade Inquiries — City Tiles" },
      {
        property: "og:description",
        content: "Project rates, credit terms and site delivery for trade buyers.",
      },
    ],
  }),
  component: TradePage,
});

const BENEFITS = [
  ["Project rates", "Tiered pricing that drops with volume, quoted per project rather than per piece."],
  ["Credit terms", "Running accounts with 30-day terms for established contractors."],
  ["Batch reservation", "We hold matched batches for phased sites so later floors still match."],
  ["Site delivery", "Own transport across AJK and northern Punjab, unloaded at your site."],
];

function TradePage() {
  return (
    <SiteShell>
      <PageHeader
        eyebrow="Bulk & Trade"
        title="Pricing built for contractors and builders"
        intro="Send us a bill of quantities or a rough estimate — you'll get a written quotation the same working day."
      />

      <section className="mx-auto grid max-w-7xl gap-16 px-5 py-20 lg:grid-cols-[1fr_1.1fr]">
        <div>
          <p className="eyebrow text-brass">What trade accounts get</p>
          <div className="mt-8 space-y-8">
            {BENEFITS.map(([title, text]) => (
              <div key={title} className="border-t border-border pt-5">
                <h3 className="text-2xl">{title}</h3>
                <p className="mt-2 text-sm text-muted-foreground">{text}</p>
              </div>
            ))}
          </div>
          <div className="mt-10 border border-border bg-card p-6">
            <p className="eyebrow text-brass">Prefer to talk?</p>
            <p className="mt-3 text-sm text-muted-foreground">
              Call {BUSINESS.phone} or message us on WhatsApp with your quantities and site
              location.
            </p>
          </div>
        </div>

        <div className="border border-border bg-card p-8 md:p-10">
          <h2 className="text-3xl">Request bulk pricing</h2>
          <p className="mt-2 text-sm text-muted-foreground">
            Include product, quantity, size/finish and delivery location.
          </p>
          <div className="mt-8">
            <InquiryForm inquiryType="bulk" showCompany submitLabel="Request Quotation" />
          </div>
        </div>
      </section>
    </SiteShell>
  );
}
