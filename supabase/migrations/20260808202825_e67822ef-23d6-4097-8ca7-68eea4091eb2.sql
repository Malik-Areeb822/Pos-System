ALTER TABLE public.products ADD COLUMN IF NOT EXISTS pieces_per_carton integer;
UPDATE public.products SET pieces_per_carton = 8 WHERE category = 'tiles' AND pieces_per_carton IS NULL;