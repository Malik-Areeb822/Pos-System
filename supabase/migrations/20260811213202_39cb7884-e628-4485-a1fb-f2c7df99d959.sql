CREATE SCHEMA IF NOT EXISTS private;
REVOKE ALL ON SCHEMA private FROM PUBLIC, anon, authenticated;
GRANT USAGE ON SCHEMA private TO authenticated, service_role;

CREATE OR REPLACE FUNCTION private.has_role(_user_id uuid, _role public.app_role)
RETURNS boolean
LANGUAGE sql
STABLE SECURITY DEFINER
SET search_path TO 'public'
AS $function$
  select exists (select 1 from public.user_roles where user_id = _user_id and role = _role)
$function$;

CREATE OR REPLACE FUNCTION private.is_staff(_user_id uuid)
RETURNS boolean
LANGUAGE sql
STABLE SECURITY DEFINER
SET search_path TO 'public'
AS $function$
  select exists (
    select 1 from public.user_roles
    where user_id = _user_id
      and role in ('admin'::public.app_role, 'cashier'::public.app_role)
  )
$function$;

REVOKE ALL ON FUNCTION private.has_role(uuid, public.app_role) FROM PUBLIC, anon;
REVOKE ALL ON FUNCTION private.is_staff(uuid) FROM PUBLIC, anon;
GRANT EXECUTE ON FUNCTION private.has_role(uuid, public.app_role) TO authenticated, service_role;
GRANT EXECUTE ON FUNCTION private.is_staff(uuid) TO authenticated, service_role;

-- customers
DROP POLICY "admin delete customers" ON public.customers;
DROP POLICY "staff insert customers" ON public.customers;
DROP POLICY "staff read customers" ON public.customers;
DROP POLICY "staff update customers" ON public.customers;
CREATE POLICY "admin delete customers" ON public.customers FOR DELETE TO authenticated USING (private.has_role(auth.uid(), 'admin'));
CREATE POLICY "staff insert customers" ON public.customers FOR INSERT TO authenticated WITH CHECK (private.is_staff(auth.uid()));
CREATE POLICY "staff read customers" ON public.customers FOR SELECT TO authenticated USING (private.is_staff(auth.uid()));
CREATE POLICY "staff update customers" ON public.customers FOR UPDATE TO authenticated USING (private.is_staff(auth.uid())) WITH CHECK (private.is_staff(auth.uid()));

-- inquiries
DROP POLICY "staff read inquiries" ON public.inquiries;
CREATE POLICY "staff read inquiries" ON public.inquiries FOR SELECT TO authenticated USING (private.is_staff(auth.uid()));

-- invoice_items
DROP POLICY "admin delete items" ON public.invoice_items;
DROP POLICY "staff insert items" ON public.invoice_items;
DROP POLICY "staff read items" ON public.invoice_items;
CREATE POLICY "admin delete items" ON public.invoice_items FOR DELETE TO authenticated USING (private.has_role(auth.uid(), 'admin'));
CREATE POLICY "staff insert items" ON public.invoice_items FOR INSERT TO authenticated WITH CHECK (private.is_staff(auth.uid()));
CREATE POLICY "staff read items" ON public.invoice_items FOR SELECT TO authenticated USING (private.is_staff(auth.uid()));

-- invoices
DROP POLICY "admin delete invoices" ON public.invoices;
DROP POLICY "staff insert invoices" ON public.invoices;
DROP POLICY "staff read invoices" ON public.invoices;
DROP POLICY "staff update invoices" ON public.invoices;
CREATE POLICY "admin delete invoices" ON public.invoices FOR DELETE TO authenticated USING (private.has_role(auth.uid(), 'admin'));
CREATE POLICY "staff insert invoices" ON public.invoices FOR INSERT TO authenticated WITH CHECK (private.is_staff(auth.uid()) AND created_by = auth.uid());
CREATE POLICY "staff read invoices" ON public.invoices FOR SELECT TO authenticated USING (private.is_staff(auth.uid()));
CREATE POLICY "staff update invoices" ON public.invoices FOR UPDATE TO authenticated USING (private.is_staff(auth.uid())) WITH CHECK (private.is_staff(auth.uid()));

-- products
DROP POLICY "admin delete" ON public.products;
DROP POLICY "staff read all" ON public.products;
DROP POLICY "staff update" ON public.products;
DROP POLICY "staff write" ON public.products;
CREATE POLICY "admin delete" ON public.products FOR DELETE TO authenticated USING (private.has_role(auth.uid(), 'admin'));
CREATE POLICY "staff read all" ON public.products FOR SELECT TO authenticated USING (private.is_staff(auth.uid()) OR is_published);
CREATE POLICY "staff update" ON public.products FOR UPDATE TO authenticated USING (private.is_staff(auth.uid())) WITH CHECK (private.is_staff(auth.uid()));
CREATE POLICY "staff write" ON public.products FOR INSERT TO authenticated WITH CHECK (private.is_staff(auth.uid()));

-- user_roles
DROP POLICY "admins manage roles" ON public.user_roles;
DROP POLICY "read own roles" ON public.user_roles;
CREATE POLICY "admins manage roles" ON public.user_roles FOR ALL TO authenticated USING (private.has_role(auth.uid(), 'admin')) WITH CHECK (private.has_role(auth.uid(), 'admin'));
CREATE POLICY "read own roles" ON public.user_roles FOR SELECT TO authenticated USING (auth.uid() = user_id OR private.has_role(auth.uid(), 'admin'));

DROP FUNCTION IF EXISTS public.is_staff(uuid);
DROP FUNCTION IF EXISTS public.has_role(uuid, public.app_role);