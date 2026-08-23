import { createFileRoute } from "@tanstack/react-router";
import { MapPin, Phone, Mail, Clock, MessageCircle } from "lucide-react";

import { SiteShell, PageHeader, BUSINESS } from "@/components/website/SiteShell";
import { InquiryForm } from "@/components/website/InquiryForm";

export const Route = createFileRoute("/website/contact")({
  head: () => ({
    meta: [
      { title: "Contact & Yard Location — City Tiles" },
      {
        name: "description",
        content:
          "Call, WhatsApp or visit City Tiles on Mansehra Road, Abbottabad, Khyber Pakhtunkhwa. Send an inquiry and we reply the same day.",
      },
      { property: "og:title", content: "Contact City Tiles" },
      {
        property: "og:description",
        content: "Phone, WhatsApp, yard location and inquiry form.",
      },
    ],
  }),
  component: ContactPage,
});

function ContactPage() {
  return (
    <SiteShell>
      <PageHeader
        eyebrow="Contact"
        title="Come see the stone in person"
        intro="Samples are always on display at the yard. Bring your drawings and we'll work out quantities with you."
      />

      <section className="mx-auto grid max-w-7xl gap-16 px-5 py-20 lg:grid-cols-[0.9fr_1.1fr]">
        <div className="space-y-8">
          {[
            { icon: MapPin, label: "Yard & factory", value: BUSINESS.address },
            { icon: Phone, label: "Phone", value: BUSINESS.phone, href: BUSINESS.phoneHref },
            {
              icon: MessageCircle,
              label: "WhatsApp",
              value: "Message our sales desk",
              href: BUSINESS.whatsapp,
            },
            { icon: Mail, label: "Email", value: BUSINESS.email, href: `mailto:${BUSINESS.email}` },
            { icon: Clock, label: "Opening hours", value: BUSINESS.hours },
          ].map((item) => (
            <div key={item.label} className="flex gap-4 border-t border-border pt-5">
              <item.icon className="mt-1 size-5 shrink-0 text-brass" />
              <div>
                <p className="eyebrow text-muted-foreground">{item.label}</p>
                {item.href ? (
                  <a href={item.href} className="mt-1 block text-lg hover:text-brass">
                    {item.value}
                  </a>
                ) : (
                  <p className="mt-1 text-lg">{item.value}</p>
                )}
              </div>
            </div>
          ))}

          <div className="overflow-hidden border border-border">
            <iframe
              title="Map showing City Tiles, Abbottabad"
              src="https://www.google.com/maps?q=34.1576975,73.2551814&z=17&output=embed"
              className="h-72 w-full"
              loading="lazy"
              referrerPolicy="no-referrer-when-downgrade"
            />
          </div>
        </div>

        <div className="border border-border bg-card p-8 md:p-10">
          <h2 className="text-3xl">Send an inquiry</h2>
          <p className="mt-2 text-sm text-muted-foreground">
            Tell us what you're building and we'll come back with options and prices.
          </p>
          <div className="mt-8">
            <InquiryForm inquiryType="contact" />
          </div>
        </div>
      </section>
    </SiteShell>
  );
}
