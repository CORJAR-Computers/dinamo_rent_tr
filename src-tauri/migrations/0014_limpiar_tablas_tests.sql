-- 0014_limpiar_tablas_tests.sql
-- Elimina tablas residuales de sesiones de test manuales (isql) que quedaron
-- en la BD de desarrollo y NO pertenecen al esquema de la app. Detectadas en
-- la auditoría gstat -i del 09-08; esquema exacto recuperado de la versión
-- commiteada de la BD (git HEAD), donde todavía existían:
--   - PROBE_T (H, D)                 — 4 filas
--   - T2      (ID, ANIO, SEQ)        — 3 filas
--   - T_TEST  (ID, ANIO, SEQ)        — 3 filas
--
-- Seguridad verificada sobre la BD real (catálogo RDB$):
--   - 0 FK entrantes (ninguna tabla de la app las referencia) y 0 FK salientes.
--   - Sin triggers, sin índices no-sistema, sin generadores y sin dependencias
--     (RDB$DEPENDENCIES vacío).
--   - El código del repo no las usa (solo aparecen en la documentación de
--     ejemplo embebida de Firebird, ajena a la BD).
--
-- SEGURIDAD ANTE COLISIONES: además de existir como TABLA real (no sistema,
-- RDB$RELATION_TYPE = 0), el guard exige que la tabla tenga EXACTAMENTE el
-- esquema residual esperado (conteo de columnas + nombres). Una tabla real
-- futura llamada igual (p.ej. una "T2" de verdad) con otro esquema NO se
-- dropea. Tradeoff residual documentado: una tabla futura que coincidiera por
-- completo con el patrón (nombre + las mismas columnas) sí se eliminaría — y
-- si además tuviera FKs entrantes, el DROP fallaría con error visible en el
-- arranque (nunca destruye datos silenciosamente).
--
-- IDEMPOTENCIA: re-ejecuciones y BDs limpias (instalaciones nuevas: 0001
-- nunca creó estas tablas) son no-op.

-- PROBE_T (H, D)
EXECUTE BLOCK
AS
BEGIN
  IF (EXISTS(SELECT 1 FROM RDB$RELATIONS
             WHERE RDB$RELATION_NAME = 'PROBE_T' AND RDB$SYSTEM_FLAG = 0
               AND RDB$RELATION_TYPE = 0)
      AND (SELECT COUNT(*) FROM RDB$RELATION_FIELDS rf
           WHERE rf.RDB$RELATION_NAME = 'PROBE_T') = 2
      AND EXISTS(SELECT 1 FROM RDB$RELATION_FIELDS rf
                 WHERE rf.RDB$RELATION_NAME = 'PROBE_T' AND rf.RDB$FIELD_NAME = 'H')
      AND EXISTS(SELECT 1 FROM RDB$RELATION_FIELDS rf
                 WHERE rf.RDB$RELATION_NAME = 'PROBE_T' AND rf.RDB$FIELD_NAME = 'D')) THEN
    EXECUTE STATEMENT 'DROP TABLE PROBE_T';
END;

-- T2 (ID, ANIO, SEQ)
EXECUTE BLOCK
AS
BEGIN
  IF (EXISTS(SELECT 1 FROM RDB$RELATIONS
             WHERE RDB$RELATION_NAME = 'T2' AND RDB$SYSTEM_FLAG = 0
               AND RDB$RELATION_TYPE = 0)
      AND (SELECT COUNT(*) FROM RDB$RELATION_FIELDS rf
           WHERE rf.RDB$RELATION_NAME = 'T2') = 3
      AND EXISTS(SELECT 1 FROM RDB$RELATION_FIELDS rf
                 WHERE rf.RDB$RELATION_NAME = 'T2' AND rf.RDB$FIELD_NAME = 'ID')
      AND EXISTS(SELECT 1 FROM RDB$RELATION_FIELDS rf
                 WHERE rf.RDB$RELATION_NAME = 'T2' AND rf.RDB$FIELD_NAME = 'ANIO')
      AND EXISTS(SELECT 1 FROM RDB$RELATION_FIELDS rf
                 WHERE rf.RDB$RELATION_NAME = 'T2' AND rf.RDB$FIELD_NAME = 'SEQ')) THEN
    EXECUTE STATEMENT 'DROP TABLE T2';
END;

-- T_TEST (ID, ANIO, SEQ)
EXECUTE BLOCK
AS
BEGIN
  IF (EXISTS(SELECT 1 FROM RDB$RELATIONS
             WHERE RDB$RELATION_NAME = 'T_TEST' AND RDB$SYSTEM_FLAG = 0
               AND RDB$RELATION_TYPE = 0)
      AND (SELECT COUNT(*) FROM RDB$RELATION_FIELDS rf
           WHERE rf.RDB$RELATION_NAME = 'T_TEST') = 3
      AND EXISTS(SELECT 1 FROM RDB$RELATION_FIELDS rf
                 WHERE rf.RDB$RELATION_NAME = 'T_TEST' AND rf.RDB$FIELD_NAME = 'ID')
      AND EXISTS(SELECT 1 FROM RDB$RELATION_FIELDS rf
                 WHERE rf.RDB$RELATION_NAME = 'T_TEST' AND rf.RDB$FIELD_NAME = 'ANIO')
      AND EXISTS(SELECT 1 FROM RDB$RELATION_FIELDS rf
                 WHERE rf.RDB$RELATION_NAME = 'T_TEST' AND rf.RDB$FIELD_NAME = 'SEQ')) THEN
    EXECUTE STATEMENT 'DROP TABLE T_TEST';
END;
