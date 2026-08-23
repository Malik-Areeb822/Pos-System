-- 004_seed_data.sql
-- 20 sample products across 4 categories
-- Prices in PKR whole rupees (integer)

-- MARBLE (5 products)
INSERT OR IGNORE INTO products (id, name, sku, category, description, color, size, finish, unit, price, pieces_per_carton, stock_qty, low_stock_threshold, image_url, is_published) VALUES
('prod-mar-001', 'Carrara White 60x60', 'MAR-CAR-6060', 'marble', 'Premium Italian Carrara white marble tiles', 'White', '60x60 cm', 'Polished', 'sqm', 1250000, NULL, 25, 5, NULL, 1),
('prod-mar-002', 'Calacatta Gold 60x120', 'MAR-CAL-60120', 'marble', 'Luxury Calacatta gold marble with gold veining', 'White/Gold', '60x120 cm', 'Polished', 'sqm', 2100000, NULL, 15, 3, NULL, 1),
('prod-mar-003', 'Statuario 30x60', 'MAR-STA-3060', 'marble', 'Classic Statuario marble with bold grey veining', 'White/Grey', '30x60 cm', 'Honed', 'sqm', 1800000, NULL, 20, 5, NULL, 1),
('prod-mar-004', 'Emperador Dark 60x60', 'MAR-EMP-6060', 'marble', 'Rich dark brown Emperador marble', 'Dark Brown', '60x60 cm', 'Polished', 'sqm', 950000, NULL, 30, 5, NULL, 1),
('prod-mar-005', 'Crema Marfil 40x40', 'MAR-CRE-4040', 'marble', 'Warm beige Crema Marfil limestone', 'Beige', '40x40 cm', 'Polished', 'sqm', 750000, NULL, 40, 8, NULL, 1);

-- TILES (5 products)
INSERT OR IGNORE INTO products (id, name, sku, category, description, color, size, finish, unit, price, pieces_per_carton, stock_qty, low_stock_threshold, image_url, is_published) VALUES
('prod-til-001', 'Porcelain Wood 20x120', 'TIL-WOOD-20120', 'tiles', 'Wood-look porcelain planks', 'Oak', '20x120 cm', 'Matte', 'sqm', 320000, 6, 100, 15, NULL, 1),
('prod-til-002', 'Concrete Grey 60x60', 'TIL-CON-6060', 'tiles', 'Industrial concrete-look porcelain', 'Grey', '60x60 cm', 'Matte', 'sqm', 280000, 4, 80, 10, NULL, 1),
('prod-til-003', 'Metro White 10x20', 'TIL-MET-1020', 'tiles', 'Classic metro/subway tiles', 'White', '10x20 cm', 'Glossy', 'sqm', 180000, 25, 200, 30, NULL, 1),
('prod-til-004', 'Hexagon Black 26x30', 'TIL-HEX-2630', 'tiles', 'Geometric hexagon mosaic tiles', 'Black', '26x30 cm', 'Matte', 'sqm', 450000, 11, 50, 8, NULL, 1),
('prod-til-005', 'Terrazzo Mix 60x60', 'TIL-TER-6060', 'tiles', 'Terrazzo-style porcelain with chips', 'Multi', '60x60 cm', 'Polished', 'sqm', 380000, 4, 60, 10, NULL, 1);

-- CHIPS (5 products)
INSERT OR IGNORE INTO products (id, name, sku, category, description, color, size, finish, unit, price, pieces_per_carton, stock_qty, low_stock_threshold, image_url, is_published) VALUES
('prod-chp-001', 'White Marble Chips 3-5mm', 'CHP-WHT-35', 'chips', 'Premium white marble chips for terrazzo', 'White', '3-5 mm', 'Natural', 'kg', 4500, NULL, 500, 50, NULL, 1),
('prod-chp-002', 'Black Marble Chips 5-8mm', 'CHP-BLK-58', 'chips', 'Black marble chips for contrast flooring', 'Black', '5-8 mm', 'Natural', 'kg', 5200, NULL, 400, 50, NULL, 1),
('prod-chp-003', 'Grey Quartz Chips 2-4mm', 'CHP-GRY-24', 'chips', 'Grey quartz chips for epoxy flooring', 'Grey', '2-4 mm', 'Natural', 'kg', 3800, NULL, 600, 60, NULL, 1),
('prod-chp-004', 'Mixed Terrazzo 4-6mm', 'CHP-MIX-46', 'chips', 'Multi-color terrazzo chip blend', 'Mixed', '4-6 mm', 'Natural', 'kg', 4200, NULL, 300, 40, NULL, 1),
('prod-chp-005', 'Pink Onyx Chips 3-5mm', 'CHP-PNK-35', 'chips', 'Rare pink onyx chips for feature floors', 'Pink', '3-5 mm', 'Natural', 'kg', 8500, NULL, 100, 15, NULL, 1);

-- SANITARY (5 products)
INSERT OR IGNORE INTO products (id, name, sku, category, description, color, size, finish, unit, price, pieces_per_carton, stock_qty, low_stock_threshold, image_url, is_published) VALUES
('prod-san-001', 'Wall-Hung WC Set', 'SAN-WC-001', 'sanitary', 'Rimless wall-hung toilet with soft-close seat', 'White', '54x36 cm', 'Glossy', 'pcs', 4500000, NULL, 10, 2, NULL, 1),
('prod-san-002', 'Countertop Basin 50cm', 'SAN-BAS-050', 'sanitary', 'Above-counter vessel basin', 'White', '50x40 cm', 'Glossy', 'pcs', 1800000, NULL, 25, 3, NULL, 1),
('prod-san-003', 'Single-Lever Mixer Chrome', 'SAN-MIX-001', 'sanitary', 'Basin mixer with ceramic cartridge', 'Chrome', 'Standard', 'Polished', 'pcs', 850000, NULL, 40, 5, NULL, 1),
('prod-san-004', 'Rain Shower Head 25cm', 'SAN-SHW-025', 'sanitary', 'Square rain shower head 250mm', 'Chrome', '25 cm', 'Polished', 'pcs', 650000, NULL, 30, 4, NULL, 1),
('prod-san-005', 'Bottle Trap Chrome', 'SAN-BTL-001', 'sanitary', 'Minimalist bottle trap for basins', 'Chrome', 'Standard', 'Polished', 'pcs', 350000, NULL, 50, 8, NULL, 1);