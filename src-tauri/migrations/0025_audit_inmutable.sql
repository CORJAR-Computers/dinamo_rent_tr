-- 0025_audit_inmutable.sql
-- Hace la tabla `auditoria` append-only (inmutable): bloquea UPDATE y DELETE
-- mediante triggers que lanzan EXCEPTION. Esto garantiza no-repudio y cumple
-- con requisitos de trazabilidad (Ley 1581 Colombia, SOX-like para ERPs).
--
-- Fundamento: la auditoría registra quién hizo qué y cuándo. Si un admin con
-- acceso SQL directo puede UPDATE o DELETE filas de auditoria, el log pierde
-- valor probatorio. Con estos triggers, la única forma de modificar el log
-- es desactivando los triggers a propósito (acción que deja rastro en
-- RDB$TRIGGERS.RDB$TRIGGER_INACTIVE).
--
-- IDEMPOTENCIA: cada trigger se crea solo si no existe (guard en RDB$TRIGGERS).
-- Patrón EXECUTE BLOCK + EXECUTE STATEMENT (compatible con el runner que
-- divide por ';' y no soporta bloques PSQL con ';' internos).

-- ── Trigger BEFORE UPDATE: bloquea cualquier UPDATE sobre auditoria ──
EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$TRIGGERS WHERE RDB$TRIGGER_NAME = 'TRG_AUDITORIA_NO_UPDATE')) THEN
    EXECUTE STATEMENT 'CREATE TRIGGER trg_auditoria_no_update FOR auditoria
      ACTIVE BEFORE UPDATE POSITION 0
      AS
      BEGIN
        EXCEPTION ''auditoria es append-only: no se puede modificar (trigger trg_auditoria_no_update)'';
      END';
END;

-- ── Trigger BEFORE DELETE: bloquea cualquier DELETE sobre auditoria ──
EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$TRIGGERS WHERE RDB$TRIGGER_NAME = 'TRG_AUDITORIA_NO_DELETE')) THEN
    EXECUTE STATEMENT 'CREATE TRIGGER trg_auditoria_no_delete FOR auditoria
      ACTIVE BEFORE DELETE POSITION 0
      AS
      BEGIN
        EXCEPTION ''auditoria es append-only: no se puede eliminar (trigger trg_auditoria_no_delete)'';
      END';
END;

-- ── Comentario documental (sin efecto funcional) ──
-- Para mantenimiento legítimo (purgar auditoría antigua por retención):
--   1. Temporalmente desactivar: ALTER TRIGGER trg_auditoria_no_delete INACTIVE;
--   2. Hacer el DELETE con WHERE fecha < CAST('2020-01-01' AS DATE);
--   3. Reactivar: ALTER TRIGGER trg_auditoria_no_delete ACTIVE;
-- Esto deja rastro en RDB$TRIGGERS.RDB$TRIGGER_INACTIVE (timestamp de modificación)
-- y debería documentarse en un procedimiento operacional separado.
