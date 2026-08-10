-- 0015_comparendo_numero_simit.sql
-- Columna `numero_comparendo`: número oficial del comparendo (fuente SIMIT o
-- registro manual). Permite al Agente SIMIT deduplicar entre sincronizaciones
-- (cada 2 horas) sin importar si el monto o la fecha cambian en el SIMIT.
--
-- IDEMPOTENCIA (mismo patrón que 0001-0014): los guards revisan el catálogo
-- RDB$ antes de ejecutar el DDL, así que reintentos e instalaciones nuevas
-- (donde 0001 ya creó la columna como parte del esquema base NO es el caso:
-- la columna solo existe aquí) son seguros. Nota: la migración se aplica a la
-- vez sobre BDs existentes y nuevas; 0001 NO crea esta columna.

-- 1) Columna numero_comparendo (nullable, VARCHAR(30))
EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$RELATION_FIELDS rf
                 WHERE rf.RDB$RELATION_NAME = 'COMPARENDOS'
                   AND rf.RDB$FIELD_NAME = 'NUMERO_COMPARENDO')) THEN
    EXECUTE STATEMENT 'ALTER TABLE comparendos ADD numero_comparendo VARCHAR(30)';
END;

-- 2) Índice de búsqueda por número (deduplicación del agente)
EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$INDICES
                 WHERE RDB$INDEX_NAME = 'IX_COMPARENDOS_NUMERO')) THEN
    EXECUTE STATEMENT 'CREATE INDEX IX_COMPARENDOS_NUMERO ON comparendos(numero_comparendo)';
END;
