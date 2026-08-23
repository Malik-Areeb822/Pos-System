# Bagh Stones & Supplies

Bagh Marble and Tiles Factory

Build a premium business website with an integrated POS (point of sale) system for Bagh Marble and Tiles Factory, a company that sells natural marble, tiles, ground flooring chips, and sanitary products.

Business Context

Bagh Marble and Tiles Factory supplies:

Marble — multiple types/slabs for flooring, walls, counters

Tiles — various sizes, finishes, and patterns

Ground flooring chips — sold by color (used for terrazzo-style flooring)

Sanitary products — for homes, washrooms, and TB (toilet/bathroom) lounges

The site needs to feel like a trusted, established stone & construction materials supplier — premium, tactile, and credible to both homeowners and contractors/builders who buy in bulk.

Design Direction

Use a color palette inspired by natural stone — avoid generic tech-startup blues/purples entirely.

Base palette: warm ivory, soft limestone beige, charcoal grey, deep marble black

Accent: brushed gold or brass (for CTAs, icons, highlights) — evokes premium finish

Optional secondary accent: terracotta or muted emerald, used sparingly for tile/sanitary sections to visually differentiate categories

Typography: a clean modern serif or high-contrast sans for headings (stone/luxury feel), paired with a simple readable sans for body text

Generous whitespace, large high-quality product imagery, subtle stone-texture backgrounds or gradients rather than flat color blocks

Should feel closer to a boutique interior/architecture brand than a typical e-commerce template

Site Structure (Public-Facing)

Home — hero banner (factory/product imagery), brief intro, category highlights (Marble / Tiles / Chips / Sanitary), trust signals (years in business, bulk supply, quality assurance)

Product Catalog — organized by category, each with:

Filter by type, color, size/finish

Grid of product cards (image, name, key specs, "Request Quote" or "Add to Order" button)

Product detail page (large images, specifications, available colors/finishes, related products)

About — factory story, manufacturing/sourcing process, quality standards

Bulk/Trade Inquiries — dedicated page for contractors/builders to request bulk pricing

Contact — location, phone, WhatsApp, inquiry form, map embed

POS System (Admin Dashboard, Authenticated)

Build a separate authenticated admin area with:

Inventory management — add/edit products across all 4 categories, track stock by variant (color, size, finish), low-stock alerts

Sales entry / billing — quick point-of-sale screen to create an invoice: search product, add quantity, auto-calculate totals, apply discounts, select payment method (cash/bank/credit)

Invoice generation — printable/downloadable invoice with business branding

Customer records — basic CRM: name, contact, order history, outstanding balance for credit customers

Sales reports/dashboard — daily/weekly/monthly sales totals, best-selling products, stock value overview

Role-based access — admin vs. staff/cashier roles with different permissions

Technical Notes

Use Supabase (via Lovable's native integration) for the database: products, inventory, customers, invoices, sales

Responsive design — the POS screen especially should work well on tablet, since it may be used at a sales counter

Keep the public site and admin dashboard visually distinct: public site = premium/marketing tone, admin dashboard = clean/functional/data-dense

Tone

Professional, premium, and trustworthy — this is a client-facing deliverable, so prioritize a polished first impression over feature density on the initial buil

This project was built with [Lovable](https://lovable.dev).

**Live app**: https://stone-flow-pos.lovable.app

## Build with Lovable

Continue developing this project in the [Lovable editor](https://lovable.dev/projects/9e124b98-ea6f-4ef1-9366-7a289af473da).

- **Ship faster**: describe what you want to build and Lovable handles the code.
- **Stay in sync**: every change made in Lovable is committed straight to this repository.
- **Full ownership**: this code is yours. Push to `main` on GitHub and your changes sync back into Lovable, ready for your next prompt.

## Development

Prefer working locally? You need Node.js and npm — [install with nvm](https://github.com/nvm-sh/nvm#installing-and-updating).

```sh
git clone <this-repository-url>
cd <repository-name>
npm i
npm run dev
```
