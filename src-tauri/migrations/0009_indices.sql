-- 0009_indices.sql
-- Índices faltantes detectados en el análisis de rendimiento.
--
-- Verificación de índices solicitados vs. existentes (0001 + 0002):
--
-- 1. inspecciones.id_renta — NO existe índice en 0001 ni 0002. Se crea.
--    Justificación: cada renta consulta sus inspecciones via
--    `WHERE id_renta = ?` (repositories/renta.rs::inspecciones_de). Sin índice,
--    Firebird hace un full table scan de inspecciones por cada renta.
--
-- 2. rentas(fecha_recogida, fecha_retorno) — YA EXISTE como idx_rentas_fechas
--    (0001_initial_schema.sql línea 178). No se recrea.
--
-- 3. mantenimiento_vehiculos(fecha) — la columna se llama pieza_varias_fecha
--    (no 'fecha'), y YA EXISTE como IDX_MANTENIMIENTO_FECHA
--    (0002_indices_optimizacion.sql línea 11). No se recrea.
--
-- 4. auditoria(usuario, fecha_hora) — la columna se llama fecha (no
--    'fecha_hora'), y YA EXISTE como ix_auditoria_usuario_fecha
--    (0001_initial_schema.sql línea 40). No se recrea.
--
-- IDEMPOTENCIA: el CREATE INDEX va guardado contra RDB$INDICES (EXECUTE BLOCK
-- + EXECUTE STATEMENT) para auto-reparar BD con aplicación parcial previa.

EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$INDICES WHERE RDB$INDEX_NAME = 'IX_INSPECCIONES_ID_RENTA')) THEN
    EXECUTE STATEMENT 'CREATE INDEX ix_inspecciones_id_renta ON inspecciones (id_renta)';
END;

-- La deduplicación de estos índices redundantes (ix_ vs IDX_ sobre las mismas
-- columnas) se hace en 0010_dedup_indices.sql: DROP condicional que solo
-- elimina el IX_* de 0001 si el IDX_* de 0002 existe (verificado con SET PLAN
-- ON: el planner usa el índice de la FK a AUTOS, no estos índices).
