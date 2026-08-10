-- 0013_consolidar_indices_auditoria.sql
-- Elimina IX_AUDITORIA_USUARIO, el último índice de una columna subsumido por
-- un compuesto de prefijo izquierdo (detectado en la auditoría final del
-- catálogo post-0012): la cubre IX_AUDITORIA_USUARIO_FECHA (usuario, fecha).
--
-- Nota de impacto: las consultas de repositories/auditoria.rs sobre AUDITORIA
-- ya resuelven con PLAN (AUDITORIA NATURAL) por ser una tabla pequeña (el
-- planner lo prefiere sobre cualquier índice), así que este índice no aporta
-- ni lecturas ni planes — solo coste de escritura.
--
-- 0001 deja de crearlo (solo afecta a instalaciones nuevas); aquí se elimina
-- de las BDs existentes.
--
-- IDEMPOTENCIA/SEGURIDAD: el DROP solo se ejecuta si queda OTRO índice (no
-- sistema) de AUDITORIA con USUARIO como PRIMER segmento (RDB$FIELD_POSITION
-- = 0) y ACTIVO (RDB$INDEX_INACTIVE = 0): nunca se deja una columna sin
-- cobertura de prefijo izquierdo usable. Re-ejecuciones y BDs ya consolidadas
-- son no-op.

EXECUTE BLOCK
AS
BEGIN
  IF (EXISTS(SELECT 1 FROM RDB$INDICES WHERE RDB$INDEX_NAME = 'IX_AUDITORIA_USUARIO')
      AND EXISTS(SELECT 1 FROM RDB$INDEX_SEGMENTS s
                  JOIN RDB$INDICES i2 ON i2.RDB$INDEX_NAME = s.RDB$INDEX_NAME
                 WHERE i2.RDB$SYSTEM_FLAG = 0
                   AND i2.RDB$RELATION_NAME = 'AUDITORIA'
                   AND i2.RDB$INDEX_NAME <> 'IX_AUDITORIA_USUARIO'
                   AND s.RDB$FIELD_NAME = 'USUARIO'
                   AND s.RDB$FIELD_POSITION = 0
                   AND COALESCE(i2.RDB$INDEX_INACTIVE, 0) = 0)) THEN
    EXECUTE STATEMENT 'DROP INDEX IX_AUDITORIA_USUARIO';
END;
