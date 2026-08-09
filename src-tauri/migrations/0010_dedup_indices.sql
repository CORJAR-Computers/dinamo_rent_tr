-- 0010_dedup_indices.sql
-- Deduplica índices redundantes (misma columna, distinto nombre) detectados en
-- el TODO de 0009:
--   - rentas(placa): ix_rentas_placa (0001) vs IDX_RENTAS_PLACA (0002)
--   - mantenimiento_vehiculos(placa): ix_mantenimiento_vehiculos_placa (0001)
--     vs IDX_MANTENIMIENTO_PLACA (0002)
--
-- Se conserva el índice IDX_* (serie de optimización de 0002) y se elimina el
-- IX_* de 0001.
--
-- Seguridad del DROP (verificado con SET PLAN ON sobre una copia de la BD de
-- desarrollo): el planner NO usa los índices IX_/IDX_ sobre placa — para
-- `WHERE placa = ?` usa el índice automático de la FK a AUTOS:
--   PLAN (RENTAS INDEX (RDB$FOREIGN38))
--   PLAN (MANTENIMIENTO_VEHICULOS INDEX (RDB$FOREIGN34))
-- Eliminar el duplicado no cambia ningún plan de ejecución.
--
-- IDEMPOTENCIA: el DROP solo se ejecuta si AMBOS índices existen (el duplicado
-- real). BDs ya consolidadas, BDs con un solo índice (p.ej. la BD de
-- desarrollo, donde falta IDX_RENTAS_PLACA) o re-ejecuciones quedan intactas.

EXECUTE BLOCK
AS
BEGIN
  IF (EXISTS(SELECT 1 FROM RDB$INDICES WHERE RDB$INDEX_NAME = 'IX_RENTAS_PLACA')
      AND EXISTS(SELECT 1 FROM RDB$INDICES WHERE RDB$INDEX_NAME = 'IDX_RENTAS_PLACA')) THEN
    EXECUTE STATEMENT 'DROP INDEX ix_rentas_placa';
END;

EXECUTE BLOCK
AS
BEGIN
  IF (EXISTS(SELECT 1 FROM RDB$INDICES WHERE RDB$INDEX_NAME = 'IX_MANTENIMIENTO_VEHICULOS_PLACA')
      AND EXISTS(SELECT 1 FROM RDB$INDICES WHERE RDB$INDEX_NAME = 'IDX_MANTENIMIENTO_PLACA')) THEN
    EXECUTE STATEMENT 'DROP INDEX ix_mantenimiento_vehiculos_placa';
END;
