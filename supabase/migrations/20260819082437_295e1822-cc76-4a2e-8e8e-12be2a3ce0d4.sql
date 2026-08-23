-- 1. Account status enum + profile fields
DO $$ BEGIN
  CREATE TYPE public.account_status AS ENUM ('pending','approved','rejected','suspended');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

ALTER TABLE public.profiles
  ADD COLUMN IF NOT EXISTS email text,
  ADD COLUMN IF NOT EXISTS phone text,
  ADD COLUMN IF NOT EXISTS employee_id text,
  ADD COLUMN IF NOT EXISTS status public.account_status NOT NULL DEFAULT 'pending',
  ADD COLUMN IF NOT EXISTS approved_at timestamptz,
  ADD COLUMN IF NOT EXISTS approved_by uuid,
  ADD COLUMN IF NOT EXISTS rejected_at timestamptz,
  ADD COLUMN IF NOT EXISTS rejected_by uuid;

-- Preserve existing users: everyone who already has a role keeps working
UPDATE public.profiles p
SET status = 'approved', approved_at = COALESCE(p.approved_at, now())
WHERE EXISTS (SELECT 1 FROM public.user_roles r WHERE r.user_id = p.id);

UPDATE public.profiles p
SET email = u.email
FROM auth.users u
WHERE u.id = p.id AND p.email IS NULL;

-- 2. Only ONE admin may ever exist
CREATE OR REPLACE FUNCTION public.enforce_single_admin()
RETURNS trigger
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path TO 'public'
AS $$
BEGIN
  IF NEW.role = 'admin'::public.app_role THEN
    IF EXISTS (
      SELECT 1 FROM public.user_roles
      WHERE role = 'admin'::public.app_role
        AND user_id <> NEW.user_id
    ) THEN
      RAISE EXCEPTION 'An admin account already exists';
    END IF;
  END IF;
  RETURN NEW;
END;
$$;
REVOKE ALL ON FUNCTION public.enforce_single_admin() FROM PUBLIC, anon, authenticated;

DROP TRIGGER IF EXISTS enforce_single_admin_trg ON public.user_roles;
CREATE TRIGGER enforce_single_admin_trg
BEFORE INSERT OR UPDATE ON public.user_roles
FOR EACH ROW EXECUTE FUNCTION public.enforce_single_admin();

-- 3. New signups: first ever user becomes approved admin, everyone else pending cashier
CREATE OR REPLACE FUNCTION public.handle_new_user()
RETURNS trigger
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path TO 'public'
AS $$
declare
  admin_exists boolean;
begin
  select exists (select 1 from public.user_roles where role = 'admin'::public.app_role) into admin_exists;

  insert into public.profiles (id, full_name, email, phone, employee_id, status, approved_at)
  values (
    new.id,
    coalesce(new.raw_user_meta_data->>'full_name',''),
    new.email,
    nullif(new.raw_user_meta_data->>'phone',''),
    nullif(new.raw_user_meta_data->>'employee_id',''),
    case when admin_exists then 'pending'::public.account_status else 'approved'::public.account_status end,
    case when admin_exists then null else now() end
  )
  on conflict (id) do nothing;

  insert into public.user_roles (user_id, role)
  values (new.id, case when admin_exists then 'cashier'::public.app_role else 'admin'::public.app_role end)
  on conflict do nothing;
  return new;
end;
$$;
REVOKE ALL ON FUNCTION public.handle_new_user() FROM PUBLIC, anon, authenticated;

-- 4. Staff checks now require an approved account
CREATE OR REPLACE FUNCTION private.has_role(_user_id uuid, _role public.app_role)
RETURNS boolean
LANGUAGE sql
STABLE SECURITY DEFINER
SET search_path TO 'public'
AS $$
  select exists (
    select 1 from public.user_roles r
    join public.profiles p on p.id = r.user_id
    where r.user_id = _user_id and r.role = _role and p.status = 'approved'::public.account_status
  )
$$;

CREATE OR REPLACE FUNCTION private.is_staff(_user_id uuid)
RETURNS boolean
LANGUAGE sql
STABLE SECURITY DEFINER
SET search_path TO 'public'
AS $$
  select exists (
    select 1 from public.user_roles r
    join public.profiles p on p.id = r.user_id
    where r.user_id = _user_id
      and r.role in ('admin'::public.app_role, 'cashier'::public.app_role)
      and p.status = 'approved'::public.account_status
  )
$$;

-- 5. Profiles: admin can read everyone; nobody can change their own status
DROP POLICY IF EXISTS "admin read profiles" ON public.profiles;
CREATE POLICY "admin read profiles" ON public.profiles
FOR SELECT TO authenticated USING (private.has_role(auth.uid(), 'admin'));

CREATE OR REPLACE FUNCTION public.protect_profile_status()
RETURNS trigger
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path TO 'public'
AS $$
BEGIN
  IF NEW.status IS DISTINCT FROM OLD.status
     OR NEW.approved_by IS DISTINCT FROM OLD.approved_by
     OR NEW.rejected_by IS DISTINCT FROM OLD.rejected_by THEN
    IF current_setting('app.status_change', true) <> 'on' THEN
      RAISE EXCEPTION 'Account status can only be changed by an admin';
    END IF;
  END IF;
  RETURN NEW;
END;
$$;
REVOKE ALL ON FUNCTION public.protect_profile_status() FROM PUBLIC, anon, authenticated;

DROP TRIGGER IF EXISTS protect_profile_status_trg ON public.profiles;
CREATE TRIGGER protect_profile_status_trg
BEFORE UPDATE ON public.profiles
FOR EACH ROW EXECUTE FUNCTION public.protect_profile_status();

-- 6. Admin-only status management RPC
CREATE OR REPLACE FUNCTION public.set_account_status(_user_id uuid, _status public.account_status)
RETURNS void
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path TO 'public'
AS $$
BEGIN
  IF NOT private.has_role(auth.uid(), 'admin'::public.app_role) THEN
    RAISE EXCEPTION 'Only an approved admin can change account status';
  END IF;
  IF _user_id = auth.uid() THEN
    RAISE EXCEPTION 'You cannot change your own account status';
  END IF;
  IF EXISTS (SELECT 1 FROM public.user_roles WHERE user_id = _user_id AND role = 'admin'::public.app_role) THEN
    RAISE EXCEPTION 'The admin account cannot be modified';
  END IF;

  PERFORM set_config('app.status_change', 'on', true);
  UPDATE public.profiles
  SET status = _status,
      approved_at = CASE WHEN _status = 'approved' THEN now() ELSE approved_at END,
      approved_by = CASE WHEN _status = 'approved' THEN auth.uid() ELSE approved_by END,
      rejected_at = CASE WHEN _status = 'rejected' THEN now() ELSE rejected_at END,
      rejected_by = CASE WHEN _status = 'rejected' THEN auth.uid() ELSE rejected_by END
  WHERE id = _user_id;
  PERFORM set_config('app.status_change', 'off', true);
END;
$$;
REVOKE ALL ON FUNCTION public.set_account_status(uuid, public.account_status) FROM PUBLIC, anon;
GRANT EXECUTE ON FUNCTION public.set_account_status(uuid, public.account_status) TO authenticated;

-- 7. Public check used by the login screen for first-time setup
CREATE OR REPLACE FUNCTION public.admin_exists()
RETURNS boolean
LANGUAGE sql
STABLE SECURITY DEFINER
SET search_path TO 'public'
AS $$
  select exists (select 1 from public.user_roles where role = 'admin'::public.app_role)
$$;
REVOKE ALL ON FUNCTION public.admin_exists() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.admin_exists() TO anon, authenticated;
