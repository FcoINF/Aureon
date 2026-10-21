-- Aureon · Migración 0002 · Paleta monocromática
--
-- Recolorea las categorías del catálogo inicial que todavía conservan su
-- color original. Si una categoría fue personalizada por la persona usuaria,
-- su color se respeta y no se modifica.

UPDATE categories SET color = '#5A5A5A' WHERE id = 'cat-alimentacion'   AND color = '#C2703D';
UPDATE categories SET color = '#4D4D4D' WHERE id = 'cat-transporte'     AND color = '#4C7BA8';
UPDATE categories SET color = '#6B6B6B' WHERE id = 'cat-vivienda'       AND color = '#6C6F86';
UPDATE categories SET color = '#454545' WHERE id = 'cat-servicios'      AND color = '#3F8A7A';
UPDATE categories SET color = '#7A7A7A' WHERE id = 'cat-salud'          AND color = '#C05A6B';
UPDATE categories SET color = '#616161' WHERE id = 'cat-educacion'      AND color = '#7A62A8';
UPDATE categories SET color = '#8A8A8A' WHERE id = 'cat-entretenimiento' AND color = '#C08A3E';
UPDATE categories SET color = '#525252' WHERE id = 'cat-compras'        AND color = '#B0674F';
UPDATE categories SET color = '#737373' WHERE id = 'cat-suscripciones'  AND color = '#8A6F9C';
UPDATE categories SET color = '#666666' WHERE id = 'cat-tecnologia'     AND color = '#4C7BA8';
UPDATE categories SET color = '#999999' WHERE id = 'cat-ropa'           AND color = '#B0768C';
UPDATE categories SET color = '#4A4A4A' WHERE id = 'cat-ahorro'         AND color = '#3F8A7A';
UPDATE categories SET color = '#7F7F7F' WHERE id = 'cat-salud-otros'    AND color = '#7B7F8A';
UPDATE categories SET color = '#A6A6A6' WHERE id = 'cat-laboral'        AND color = '#2F7A5A';
UPDATE categories SET color = '#949494' WHERE id = 'cat-ingresos-otros' AND color = '#3F8A7A';
