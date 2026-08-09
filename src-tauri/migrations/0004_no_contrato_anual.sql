-- 0004_no_contrato_anual.sql
-- Numeración de contrato ANUAL: el no_contrato se reinicia cada año y la
-- unicidad pasa a ser por (anio_contrato, no_contrato).
--
-- Nota: los UPDATEs son deterministas (mismo resultado al re-ejecutar) y sobre
-- una instalación nueva la tabla está vacía, así que son no-op.
--
-- IDEMPOTENCIA (auto-reparación de instalaciones nuevas a medias):
--   - El ALTER de anio_contrato va guardado contra RDB$RELATION_FIELDS.
--   - El DROP de ix_rentas_no_contrato es CONDICIONAL (solo si existe).
--   - El CREATE del índice anual va guardado contra RDB$INDICES.
--   - Los SET NOT NULL van guardados contra RDB$NULL_FLAG (1 = NOT NULL), ya
--     que re-ejecutarlos sobre una columna ya NOT NULL aborta la migración.

EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$RELATION_FIELDS WHERE RDB$RELATION_NAME = 'RENTAS' AND RDB$FIELD_NAME = 'ANIO_CONTRATO')) THEN
    EXECUTE STATEMENT 'ALTER TABLE rentas ADD anio_contrato SMALLINT';
END;

UPDATE rentas SET anio_contrato = COALESCE(
    CAST(EXTRACT(YEAR FROM created_at) AS SMALLINT),
    CAST(EXTRACT(YEAR FROM fecha_recogida) AS SMALLINT)
);

-- COALESCE exige tipos comparables: fecha_recogida es DATE y created_at es
-- TIMESTAMP (Firebird 5 falla con "Datatypes are not comparable"). Se castea
-- created_at a DATE.
UPDATE rentas r SET no_contrato = (
    SELECT seq FROM (
        SELECT id, ROW_NUMBER() OVER (PARTITION BY anio_contrato ORDER BY COALESCE(fecha_recogida, CAST(created_at AS DATE)), id) AS seq
        FROM rentas
    ) x WHERE x.id = r.id
);

EXECUTE BLOCK
AS
BEGIN
  IF (EXISTS(SELECT 1 FROM RDB$INDICES WHERE RDB$INDEX_NAME = 'IX_RENTAS_NO_CONTRATO')) THEN
    EXECUTE STATEMENT 'DROP INDEX ix_rentas_no_contrato';
END;

EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$INDICES WHERE RDB$INDEX_NAME = 'IX_RENTAS_NO_CONTRATO_ANIO')) THEN
    EXECUTE STATEMENT 'CREATE UNIQUE INDEX ix_rentas_no_contrato_anio ON rentas (anio_contrato, no_contrato)';
END;

EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$RELATION_FIELDS WHERE RDB$RELATION_NAME = 'RENTAS' AND RDB$FIELD_NAME = 'ANIO_CONTRATO' AND RDB$NULL_FLAG = 1)) THEN
    EXECUTE STATEMENT 'ALTER TABLE rentas ALTER COLUMN anio_contrato SET NOT NULL';
END;

EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$RELATION_FIELDS WHERE RDB$RELATION_NAME = 'RENTAS' AND RDB$FIELD_NAME = 'NO_CONTRATO' AND RDB$NULL_FLAG = 1)) THEN
    EXECUTE STATEMENT 'ALTER TABLE rentas ALTER COLUMN no_contrato SET NOT NULL';
END;
