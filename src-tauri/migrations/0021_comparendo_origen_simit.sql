-- 0021_comparendo_origen_simit.sql
-- Procedencia persistente del comparendo para distinguir «nuevos» de «ya
-- registrados» más allá de la última sincronización en memoria:
--
--   origen            VARCHAR(10) NOT NULL DEFAULT 'Manual'
--                     'SIMIT' = importado/confirmado por el Agente SIMIT
--                     'Manual' = registrado a mano en la app
--   ultimo_visto_simit TIMESTAMP NULL
--                     cuándo el Agente SIMIT confirmó por última vez que el
--                     comparendo sigue existiendo en el portal (se toca en
--                     cada corrida, nuevo o ya registrado)
--
-- IDEMPOTENCIA (mismo patrón que 0001-0020): los guards revisan el catálogo
-- RDB$ antes del DDL; el backfill es UPDATE condicional y seguro de reaplicar.
-- ORDEN: el backfill de NULLs corre ANTES del SET NOT NULL (Firebird no deja
-- poner NOT NULL mientras existan NULLs en la columna).

-- 1) Columna origen (nullable primero: la BD puede tener filas existentes)
EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$RELATION_FIELDS rf
                 WHERE rf.RDB$RELATION_NAME = 'COMPARENDOS'
                   AND rf.RDB$FIELD_NAME = 'ORIGEN')) THEN
    EXECUTE STATEMENT 'ALTER TABLE comparendos ADD origen VARCHAR(10) DEFAULT ''Manual''';
END;

-- 2) Backfill antes del NOT NULL: NULL → 'Manual'; los ya importados por el
--    Agente SIMIT (observaciones «Importado SIMIT …») → 'SIMIT'.
UPDATE comparendos
SET origen = CASE WHEN COALESCE(observaciones, '') LIKE 'Importado SIMIT%'
                  THEN 'SIMIT' ELSE 'Manual' END
WHERE origen IS NULL;

-- 3) Columna ultimo_visto_simit (timestamp de la última confirmación SIMIT)
EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$RELATION_FIELDS rf
                 WHERE rf.RDB$RELATION_NAME = 'COMPARENDOS'
                   AND rf.RDB$FIELD_NAME = 'ULTIMO_VISTO_SIMIT')) THEN
    EXECUTE STATEMENT 'ALTER TABLE comparendos ADD ultimo_visto_simit TIMESTAMP';
END;

-- 4) Ahora sí: NOT NULL con DEFAULT (ya no quedan NULLs)
EXECUTE BLOCK
AS
BEGIN
  IF (EXISTS(SELECT 1 FROM RDB$RELATION_FIELDS rf
             WHERE rf.RDB$RELATION_NAME = 'COMPARENDOS'
               AND rf.RDB$FIELD_NAME = 'ORIGEN'
               AND rf.RDB$NULL_FLAG IS NULL)) THEN
    EXECUTE STATEMENT 'ALTER TABLE comparendos ALTER COLUMN origen SET NOT NULL';
END;

-- 5) Índice para filtrar por origen (¿cuáles vienen del SIMIT?) sin scans
EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$INDICES
                 WHERE RDB$INDEX_NAME = 'IX_COMPARENDOS_ORIGEN')) THEN
    EXECUTE STATEMENT 'CREATE INDEX IX_COMPARENDOS_ORIGEN ON comparendos(origen)';
END;
