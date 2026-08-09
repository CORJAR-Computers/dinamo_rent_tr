-- 0003_no_contrato.sql
-- Número de contrato SECUENCIAL e independiente del id de la renta.
--
-- El id de renta no se reutiliza (IDENTITY) pero salta al eliminar registros y
-- es interno; el contrato impreso (ContratoRenta) debe llevar su propia
-- numeración consecutiva global: 1, 2, 3, ... sin huecos ni duplicados.
--
-- Mecánica (sin bloques PSQL: el runner de migraciones divide por ';'):
--   1. Columna no_contrato en rentas
--   2. Generator (secuencia) GEN_RENTA_NO_CONTRATO
--   3. Backfill de las rentas existentes con GEN_ID(gen, 1): cada fila toma el
--      siguiente valor y el generator queda automáticamente en el último
--      asignado, de modo que el próximo INSERT continúa la secuencia.
--   4. Índice único (garantía extra contra duplicados).

ALTER TABLE rentas ADD no_contrato INTEGER;

CREATE GENERATOR GEN_RENTA_NO_CONTRATO;

UPDATE rentas SET no_contrato = GEN_ID(GEN_RENTA_NO_CONTRATO, 1);

CREATE UNIQUE INDEX ix_rentas_no_contrato ON rentas (no_contrato);
