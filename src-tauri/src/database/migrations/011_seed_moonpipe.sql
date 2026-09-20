-- 011_seed_moonpipe.sql
-- 20 sample products for Moon Pipe and Sanitary Store
-- Categories: sanitary (10), hardware (10)
-- Prices in PKR (integer)

-- SANITARY (10 products)
INSERT OR IGNORE INTO products (id, name, sku, category, description, color, size, finish, company, unit, price, purchase_price, pieces_per_carton, area_per_tile, stock_qty, low_stock_threshold, image_url, is_published, created_at, updated_at) VALUES
('prod-san-001', 'Wall-Hung WC Rimless', 'SAN-WC-001', 'sanitary', 'Rimless wall-hung toilet with soft-close seat', 'White', '54x36 cm', 'Glossy', 'Master Sanitary', 'pcs', 45000, 30000, NULL, NULL, 10, 2, NULL, 1, datetime('now'), datetime('now')),
('prod-san-002', 'Floor-Standing WC', 'SAN-WC-002', 'sanitary', 'Standard floor-mounted toilet with cistern', 'White', '65x38 cm', 'Glossy', 'Master Sanitary', 'pcs', 25000, 16000, NULL, NULL, 15, 3, NULL, 1, datetime('now'), datetime('now')),
('prod-san-003', 'Countertop Basin 50cm', 'SAN-BAS-050', 'sanitary', 'Above-counter vessel basin ceramic', 'White', '50x40 cm', 'Glossy', 'Master Sanitary', 'pcs', 18000, 11000, NULL, NULL, 20, 3, NULL, 1, datetime('now'), datetime('now')),
('prod-san-004', 'Pedestal Basin', 'SAN-BAS-070', 'sanitary', 'Full pedestal wash basin', 'White', '60x45 cm', 'Glossy', 'Master Sanitary', 'pcs', 12000, 7500, NULL, NULL, 25, 4, NULL, 1, datetime('now'), datetime('now')),
('prod-san-005', 'Basin Mixer Chrome', 'SAN-MIX-001', 'sanitary', 'Single-lever basin mixer with ceramic cartridge', 'Chrome', 'Standard', 'Polished', NULL, 'pcs', 8500, 5500, NULL, NULL, 40, 5, NULL, 1, datetime('now'), datetime('now')),
('prod-san-006', 'Shower Mixer Chrome', 'SAN-MIX-002', 'sanitary', 'Wall-mount shower mixer with diverter', 'Chrome', 'Standard', 'Polished', NULL, 'pcs', 7500, 4800, NULL, NULL, 30, 5, NULL, 1, datetime('now'), datetime('now')),
('prod-san-007', 'Rain Shower Head 25cm', 'SAN-SHW-025', 'sanitary', 'Square rain shower head 250mm SS304', 'Chrome', '25 cm', 'Polished', NULL, 'pcs', 6500, 4000, NULL, NULL, 35, 5, NULL, 1, datetime('now'), datetime('now')),
('prod-san-008', 'Flush Valve Concealed', 'SAN-FLV-001', 'sanitary', 'Concealed flush valve for wall-hung WC', 'Chrome', 'Standard', 'Polished', NULL, 'pcs', 4500, 2800, NULL, NULL, 20, 4, NULL, 1, datetime('now'), datetime('now')),
('prod-san-009', 'Bottle Trap Chrome', 'SAN-BTL-001', 'sanitary', 'Minimalist bottle trap for basins 32mm', 'Chrome', 'Standard', 'Polished', NULL, 'pcs', 3500, 2200, NULL, NULL, 50, 8, NULL, 1, datetime('now'), datetime('now')),
('prod-san-010', 'PPR Pipe 1 inch', 'SAN-PPR-025', 'sanitary', 'PPR hot/cold water pipe 1 inch per meter', 'Green', '1 inch', 'Matte', NULL, 'mtr', 350, 220, NULL, NULL, 200, 30, NULL, 1, datetime('now'), datetime('now'));

-- HARDWARE (10 products)
INSERT OR IGNORE INTO products (id, name, sku, category, description, color, size, finish, company, unit, price, purchase_price, pieces_per_carton, area_per_tile, stock_qty, low_stock_threshold, image_url, is_published, created_at, updated_at) VALUES
('prod-hw-001', 'Gate Valve 1 inch Brass', 'HW-VLV-025G', 'hardware', 'Brass gate valve 1 inch full port', 'Brass', '1 inch', 'Natural', NULL, 'pcs', 1200, 750, NULL, NULL, 40, 5, NULL, 1, datetime('now'), datetime('now')),
('prod-hw-002', 'Ball Valve 1 inch Brass', 'HW-VLV-025B', 'hardware', 'Brass ball valve 1 inch lever handle', 'Brass', '1 inch', 'Natural', NULL, 'pcs', 800, 500, NULL, NULL, 50, 8, NULL, 1, datetime('now'), datetime('now')),
('prod-hw-003', 'Elbow 1 inch PPR', 'HW-ELB-025P', 'hardware', 'PPR elbow fitting 90 degree 1 inch', 'Green', '1 inch', 'Matte', NULL, 'pcs', 120, 70, NULL, NULL, 200, 20, NULL, 1, datetime('now'), datetime('now')),
('prod-hw-004', 'Tee 1 inch PPR', 'HW-TEE-025P', 'hardware', 'PPR tee fitting 1 inch', 'Green', '1 inch', 'Matte', NULL, 'pcs', 150, 90, NULL, NULL, 150, 20, NULL, 1, datetime('now'), datetime('now')),
('prod-hw-005', 'Thread Seal Tape', 'HW-TAP-001', 'hardware', 'Teflon thread seal tape 12mm x 20m', 'White', '12mm', 'Natural', NULL, 'pcs', 50, 25, NULL, NULL, 300, 30, NULL, 1, datetime('now'), datetime('now')),
('prod-hw-006', 'PVC Solvent Cement 100ml', 'HW-ADH-100', 'hardware', 'PVC solvent cement for pipes 100ml', 'Clear', '100 ml', 'Natural', NULL, 'pcs', 250, 150, NULL, NULL, 60, 10, NULL, 1, datetime('now'), datetime('now')),
('prod-hw-007', 'PVC Elbow 1 inch', 'HW-ELB-025V', 'hardware', 'PVC elbow 90 degree 1 inch', 'White', '1 inch', 'Matte', NULL, 'pcs', 80, 45, NULL, NULL, 200, 20, NULL, 1, datetime('now'), datetime('now')),
('prod-hw-008', 'PVC Pipe 1 inch', 'HW-PVC-025', 'hardware', 'PVC pressure pipe 1 inch per meter', 'White', '1 inch', 'Matte', NULL, 'mtr', 200, 120, NULL, NULL, 150, 20, NULL, 1, datetime('now'), datetime('now')),
('prod-hw-009', 'Threaded Nipple 1 inch', 'HW-NPL-025', 'hardware', 'GI threaded nipple 1 inch x 3 inch', 'Silver', '1 inch', 'Natural', NULL, 'pcs', 100, 60, NULL, NULL, 100, 15, NULL, 1, datetime('now'), datetime('now')),
('prod-hw-010', 'Rubber Washer Assorted', 'HW-WSH-AST', 'hardware', 'Assorted rubber washers 10-piece pack', 'Black', 'Mixed', 'Natural', NULL, 'pkt', 60, 30, NULL, NULL, 200, 20, NULL, 1, datetime('now'), datetime('now'));
