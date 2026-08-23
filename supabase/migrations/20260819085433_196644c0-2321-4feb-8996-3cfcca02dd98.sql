create table if not exists public.returns (
  id uuid primary key default gen_random_uuid(),
  invoice_id uuid not null references public.invoices(id) on delete cascade,
  invoice_no text not null default '',
  customer_id uuid references public.customers(id),
  customer_name text not null default 'Walk-in customer',
  total numeric not null default 0,
  reason text,
  created_by uuid references auth.users(id),
  created_at timestamptz not null default now()
);

create table if not exists public.return_items (
  id uuid primary key default gen_random_uuid(),
  return_id uuid not null references public.returns(id) on delete cascade,
  product_id uuid references public.products(id),
  product_name text not null,
  unit text not null default 'unit',
  quantity numeric not null default 0,
  unit_price numeric not null default 0,
  line_total numeric not null default 0
);

grant select, insert on public.returns to authenticated;
grant delete on public.returns to authenticated;
grant all on public.returns to service_role;
grant select, insert on public.return_items to authenticated;
grant delete on public.return_items to authenticated;
grant all on public.return_items to service_role;

alter table public.returns enable row level security;
alter table public.return_items enable row level security;

create policy "staff read returns" on public.returns for select to authenticated using (private.is_staff(auth.uid()));
create policy "staff insert returns" on public.returns for insert to authenticated with check (private.is_staff(auth.uid()) and created_by = auth.uid());
create policy "admin delete returns" on public.returns for delete to authenticated using (private.has_role(auth.uid(), 'admin'::public.app_role));

create policy "staff read return items" on public.return_items for select to authenticated using (private.is_staff(auth.uid()));
create policy "staff insert return items" on public.return_items for insert to authenticated with check (private.is_staff(auth.uid()));
create policy "admin delete return items" on public.return_items for delete to authenticated using (private.has_role(auth.uid(), 'admin'::public.app_role));

create or replace function public.process_return(_invoice_id uuid, _items jsonb, _reason text default null)
returns uuid
language plpgsql
security definer
set search_path = public
as $$
declare
  v_inv public.invoices%rowtype;
  v_return_id uuid;
  v_item jsonb;
  v_line public.invoice_items%rowtype;
  v_qty numeric;
  v_already numeric;
  v_total numeric := 0;
  v_new_total numeric;
  v_new_paid numeric;
  v_old_due numeric;
  v_new_due numeric;
begin
  if not private.is_staff(auth.uid()) then
    raise exception 'Not authorised to record returns';
  end if;

  select * into v_inv from public.invoices where id = _invoice_id;
  if not found then raise exception 'Invoice not found'; end if;

  insert into public.returns (invoice_id, invoice_no, customer_id, customer_name, total, reason, created_by)
  values (_invoice_id, v_inv.invoice_no, v_inv.customer_id, v_inv.customer_name, 0, nullif(btrim(coalesce(_reason,'')),''), auth.uid())
  returning id into v_return_id;

  for v_item in select * from jsonb_array_elements(_items)
  loop
    v_qty := coalesce((v_item->>'quantity')::numeric, 0);
    continue when v_qty <= 0;

    select * into v_line from public.invoice_items
      where id = (v_item->>'invoice_item_id')::uuid and invoice_id = _invoice_id;
    if not found then raise exception 'Invoice line not found'; end if;

    select coalesce(sum(ri.quantity), 0) into v_already
    from public.return_items ri
    join public.returns r on r.id = ri.return_id
    where r.invoice_id = _invoice_id
      and coalesce(ri.product_id::text, ri.product_name) = coalesce(v_line.product_id::text, v_line.product_name)
      and ri.id is not null;

    if v_qty + v_already > v_line.quantity then
      raise exception 'Return quantity for % exceeds sold quantity', v_line.product_name;
    end if;

    insert into public.return_items (return_id, product_id, product_name, unit, quantity, unit_price, line_total)
    values (v_return_id, v_line.product_id, v_line.product_name, v_line.unit, v_qty, v_line.unit_price, v_qty * v_line.unit_price);

    v_total := v_total + v_qty * v_line.unit_price;

    if v_line.product_id is not null then
      update public.products set stock_qty = stock_qty + v_qty, updated_at = now() where id = v_line.product_id;
    end if;
  end loop;

  if v_total <= 0 then
    delete from public.returns where id = v_return_id;
    raise exception 'Enter at least one returned quantity';
  end if;

  update public.returns set total = v_total where id = v_return_id;

  v_new_total := greatest(0, v_inv.total - v_total);
  v_new_paid := least(v_inv.amount_paid, v_new_total);
  v_old_due := greatest(0, v_inv.total - v_inv.amount_paid);
  v_new_due := greatest(0, v_new_total - v_new_paid);

  update public.invoices
  set subtotal = greatest(0, subtotal - v_total),
      total = v_new_total,
      amount_paid = v_new_paid
  where id = _invoice_id;

  if v_inv.customer_id is not null then
    update public.customers
    set outstanding_balance = greatest(0, outstanding_balance + (v_new_due - v_old_due))
    where id = v_inv.customer_id;
  end if;

  return v_return_id;
end;
$$;

revoke all on function public.process_return(uuid, jsonb, text) from public, anon;
grant execute on function public.process_return(uuid, jsonb, text) to authenticated;