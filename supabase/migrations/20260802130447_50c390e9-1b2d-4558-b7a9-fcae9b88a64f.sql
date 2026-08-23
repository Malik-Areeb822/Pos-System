ALTER TABLE public.products ADD COLUMN IF NOT EXISTS barcode text;
CREATE UNIQUE INDEX IF NOT EXISTS products_barcode_key ON public.products (barcode) WHERE barcode IS NOT NULL;