-- roles
create type public.app_role as enum ('admin','cashier');
create type public.product_category as enum ('marble','tiles','chips','sanitary');
create type public.payment_method as enum ('cash','bank','credit');

create table public.profiles (
  id uuid primary key references auth.users(id) on delete cascade,
  full_name text not null default '',
  created_at timestamptz not null default now()
);
grant select, insert, update on public.profiles to authenticated;
grant all on public.profiles to service_role;
alter table public.profiles enable row level security;
create policy "own profile read" on public.profiles for select to authenticated using (auth.uid() = id);
create policy "own profile write" on public.profiles for update to authenticated using (auth.uid() = id) with check (auth.uid() = id);
create policy "own profile insert" on public.profiles for insert to authenticated with check (auth.uid() = id);

create table public.user_roles (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references auth.users(id) on delete cascade,
  role public.app_role not null,
  unique (user_id, role)
);
grant select on public.user_roles to authenticated;
grant all on public.user_roles to service_role;
alter table public.user_roles enable row level security;

create or replace function public.has_role(_user_id uuid, _role public.app_role)
returns boolean language sql stable security definer set search_path = public as $$
  select exists (select 1 from public.user_roles where user_id = _user_id and role = _role)
$$;

create or replace function public.is_staff(_user_id uuid)
returns boolean language sql stable security definer set search_path = public as $$
  select exists (select 1 from public.user_roles where user_id = _user_id)
$$;

create policy "read own roles" on public.user_roles for select to authenticated using (auth.uid() = user_id or public.has_role(auth.uid(),'admin'));
create policy "admins manage roles" on public.user_roles for all to authenticated using (public.has_role(auth.uid(),'admin')) with check (public.has_role(auth.uid(),'admin'));

create or replace function public.handle_new_user()
returns trigger language plpgsql security definer set search_path = public as $$
begin
  insert into public.profiles (id, full_name)
  values (new.id, coalesce(new.raw_user_meta_data->>'full_name',''))
  on conflict (id) do nothing;
  insert into public.user_roles (user_id, role)
  values (new.id, case when (select count(*) from public.user_roles) = 0 then 'admin'::public.app_role else 'cashier'::public.app_role end)
  on conflict do nothing;
  return new;
end;
$$;
create trigger on_auth_user_created after insert on auth.users for each row execute function public.handle_new_user();

-- products
create table public.products (
  id uuid primary key default gen_random_uuid(),
  name text not null,
  sku text unique,
  category public.product_category not null,
  description text not null default '',
  color text,
  size text,
  finish text,
  unit text not null default 'sq.ft',
  price numeric(12,2) not null default 0,
  stock_qty numeric(12,2) not null default 0,
  low_stock_threshold numeric(12,2) not null default 10,
  image_url text,
  is_published boolean not null default true,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);
grant select on public.products to anon;
grant select, insert, update, delete on public.products to authenticated;
grant all on public.products to service_role;
alter table public.products enable row level security;
create policy "public read published" on public.products for select to anon using (is_published);
create policy "staff read all" on public.products for select to authenticated using (public.is_staff(auth.uid()) or is_published);
create policy "staff write" on public.products for insert to authenticated with check (public.is_staff(auth.uid()));
create policy "staff update" on public.products for update to authenticated using (public.is_staff(auth.uid())) with check (public.is_staff(auth.uid()));
create policy "admin delete" on public.products for delete to authenticated using (public.has_role(auth.uid(),'admin'));

-- customers
create table public.customers (
  id uuid primary key default gen_random_uuid(),
  name text not null,
  phone text,
  email text,
  address text,
  outstanding_balance numeric(12,2) not null default 0,
  created_at timestamptz not null default now()
);
grant select, insert, update, delete on public.customers to authenticated;
grant all on public.customers to service_role;
alter table public.customers enable row level security;
create policy "staff read customers" on public.customers for select to authenticated using (public.is_staff(auth.uid()));
create policy "staff insert customers" on public.customers for insert to authenticated with check (public.is_staff(auth.uid()));
create policy "staff update customers" on public.customers for update to authenticated using (public.is_staff(auth.uid())) with check (public.is_staff(auth.uid()));
create policy "admin delete customers" on public.customers for delete to authenticated using (public.has_role(auth.uid(),'admin'));

-- invoices
create sequence if not exists public.invoice_seq start 1001;
create table public.invoices (
  id uuid primary key default gen_random_uuid(),
  invoice_no text not null unique default ('BMT-' || nextval('public.invoice_seq')::text),
  customer_id uuid references public.customers(id) on delete set null,
  customer_name text not null default 'Walk-in Customer',
  subtotal numeric(12,2) not null default 0,
  discount numeric(12,2) not null default 0,
  total numeric(12,2) not null default 0,
  amount_paid numeric(12,2) not null default 0,
  payment_method public.payment_method not null default 'cash',
  notes text,
  created_by uuid references auth.users(id) on delete set null,
  created_at timestamptz not null default now()
);
grant select, insert, update, delete on public.invoices to authenticated;
grant all on public.invoices to service_role;
alter table public.invoices enable row level security;
create policy "staff read invoices" on public.invoices for select to authenticated using (public.is_staff(auth.uid()));
create policy "staff insert invoices" on public.invoices for insert to authenticated with check (public.is_staff(auth.uid()) and created_by = auth.uid());
create policy "admin update invoices" on public.invoices for update to authenticated using (public.has_role(auth.uid(),'admin')) with check (public.has_role(auth.uid(),'admin'));
create policy "admin delete invoices" on public.invoices for delete to authenticated using (public.has_role(auth.uid(),'admin'));

create table public.invoice_items (
  id uuid primary key default gen_random_uuid(),
  invoice_id uuid not null references public.invoices(id) on delete cascade,
  product_id uuid references public.products(id) on delete set null,
  product_name text not null,
  unit text not null default 'unit',
  quantity numeric(12,2) not null default 1,
  unit_price numeric(12,2) not null default 0,
  line_total numeric(12,2) not null default 0
);
grant select, insert, update, delete on public.invoice_items to authenticated;
grant all on public.invoice_items to service_role;
alter table public.invoice_items enable row level security;
create policy "staff read items" on public.invoice_items for select to authenticated using (public.is_staff(auth.uid()));
create policy "staff insert items" on public.invoice_items for insert to authenticated with check (public.is_staff(auth.uid()));
create policy "admin delete items" on public.invoice_items for delete to authenticated using (public.has_role(auth.uid(),'admin'));

-- inquiries
create table public.inquiries (
  id uuid primary key default gen_random_uuid(),
  name text not null,
  phone text not null,
  email text,
  company text,
  inquiry_type text not null default 'general',
  message text not null default '',
  created_at timestamptz not null default now()
);
grant insert on public.inquiries to anon;
grant select, insert, update, delete on public.inquiries to authenticated;
grant all on public.inquiries to service_role;
alter table public.inquiries enable row level security;
create policy "anyone can submit" on public.inquiries for insert to anon with check (true);
create policy "auth can submit" on public.inquiries for insert to authenticated with check (true);
create policy "staff read inquiries" on public.inquiries for select to authenticated using (public.is_staff(auth.uid()));

insert into public.products (name, sku, category, description, color, size, finish, unit, price, stock_qty, low_stock_threshold) values
('Ziarat White Marble','MRB-001','marble','Premium Ziarat white marble slab with soft grey veining, ideal for flooring and stairs.','White','24x24 in','Polished','sq.ft',285,1200,200),
('Badal Grey Marble','MRB-002','marble','Cloud-grey marble with dramatic movement, suited to feature walls and counters.','Grey','24x36 in','Polished','sq.ft',340,780,150),
('Verona Beige Marble','MRB-003','marble','Warm beige marble with fine grain, a classic choice for living areas.','Beige','18x18 in','Honed','sq.ft',260,940,150),
('Jet Black Marble','MRB-004','marble','Deep black marble slab with a mirror polish for luxury interiors.','Black','24x24 in','Polished','sq.ft',420,410,100),
('Travertine Classic','MRB-005','marble','Natural travertine with characterful pitting, perfect for façades.','Ivory','16x16 in','Brushed','sq.ft',300,520,100),
('Porcelain Stone Look Tile','TIL-001','tiles','Large-format porcelain tile with a stone-look surface and low porosity.','Ivory','600x600 mm','Matte','sq.ft',180,2400,300),
('Glazed Wall Tile','TIL-002','tiles','Glossy glazed ceramic wall tile for kitchens and washrooms.','White','300x600 mm','Glossy','sq.ft',120,3100,400),
('Terracotta Floor Tile','TIL-003','tiles','Handmade-look terracotta tile with warm earthen tone.','Terracotta','400x400 mm','Matte','sq.ft',150,860,200),
('Charcoal Anti-Skid Tile','TIL-004','tiles','Anti-skid porcelain for outdoor terraces and wet areas.','Charcoal','300x300 mm','Textured','sq.ft',165,1450,250),
('Emerald Patterned Tile','TIL-005','tiles','Decorative patterned tile for accent walls and niches.','Emerald','200x200 mm','Satin','sq.ft',210,600,120),
('White Flooring Chips','CHP-001','chips','Graded white marble chips for terrazzo-style cast flooring.','White','6-9 mm','Crushed','kg',45,5200,800),
('Terracotta Flooring Chips','CHP-002','chips','Warm terracotta chips for decorative terrazzo mixes.','Terracotta','6-9 mm','Crushed','kg',52,3100,600),
('Emerald Flooring Chips','CHP-003','chips','Green marble chips used for feature terrazzo flooring.','Emerald','4-6 mm','Crushed','kg',58,2400,500),
('Black Flooring Chips','CHP-004','chips','Jet black chips for high-contrast terrazzo floors.','Black','6-9 mm','Crushed','kg',49,4100,700),
('Wall-Hung Commode','SAN-001','sanitary','Wall-hung ceramic commode with soft-close seat.','White','Standard','Glossy','piece',24500,42,10),
('Pedestal Wash Basin','SAN-002','sanitary','Full pedestal wash basin in vitreous china.','White','Standard','Glossy','piece',9800,65,15),
('Brass Mixer Tap','SAN-003','sanitary','Brushed brass single-lever basin mixer with ceramic cartridge.','Brass','Standard','Brushed','piece',7600,88,20),
('Shower Enclosure Kit','SAN-004','sanitary','Tempered glass shower enclosure with brass-finish frame.','Clear','900x900 mm','Brushed','set',48000,18,5),
('TB Lounge Vanity Unit','SAN-005','sanitary','Marble-top vanity unit designed for washroom and TB lounge fit-outs.','Ivory','1200 mm','Polished','piece',36500,24,6);