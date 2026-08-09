-- 0003_no_contrato.sql
-- Número de contrato SECUENCIAL e independiente del id de la renta.
--
-- El id de renta no se reutiliza (IDENTITY) pero salta al eliminar registros y
-- es interno; el contrato impreso (ContratoRenta) debe llevar su propia
-- numeración consecutiva global: 1, 2, 3, ... sin huecos ni duplicados.
--
-- Mecánica:
--   1. Columna no_contrato en rentas
--   2. Generator (secuencia) GEN_RENTA_NO_CONTRATO
--   3. Backfill de las rentas existentes con GEN_ID(gen, 1): cada fila toma el
--      siguiente valor y el generator queda automáticamente en el último
--      asignado, de modo que el próximo INSERT continúa la secuencia.
--   4. Índice único (garantía extra contra duplicados).
--
-- IDEMPOTENCIA (auto-reparación de instalaciones nuevas a medias):
--   - La columna va guardada contra RDB$RELATION_FIELDS.
--   - El generator va guardado contra RDB$GENERATORS.
--   - El backfill solo numera filas sin asignar (WHERE no_contrato IS NULL):
--     en una instalación nueva la tabla está vacía, y si un reintento encuentra
--     filas NULL las numera sin re-numerar las ya asignadas.
--   - El índice único va guardado contra RDB$INDICES.

EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$RELATION_FIELDS WHERE RDB$RELATION_NAME = 'RENTAS' AND RDB$FIELD_NAME = 'NO_CONTRATO')) THEN
    EXECUTE STATEMENT 'ALTER TABLE rentas ADD no_contrato INTEGER';
END;

EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$GENERATORS WHERE RDB$GENERATOR_NAME = 'GEN_RENTA_NO_CONTRATO')) THEN
    EXECUTE STATEMENT 'CREATE GENERATOR GEN_RENTA_NO_CONTRATO';
END;

UPDATE rentas SET no_contrato = GEN_ID(GEN_RENTA_NO_CONTRATO, 1) WHERE no_contrato IS NULL;

EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$INDICES WHERE RDB$INDEX_NAME = 'IX_RENTAS_NO_CONTRATO')) THEN
    EXECUTE STATEMENT 'CREATE UNIQUE INDEX ix_rentas_no_contrato ON rentas (no_contrato)';
END;
