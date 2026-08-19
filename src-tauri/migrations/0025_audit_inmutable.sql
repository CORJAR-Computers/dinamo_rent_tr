-- 0025_audit_inmutable.sql
-- Hace la tabla `auditoria` append-only (inmutable): bloquea UPDATE y DELETE
-- mediante triggers que lanzan EXCEPTION. Esto garantiza no-repudio y cumple
-- con requisitos de trazabilidad (Ley 1581 Colombia, SOX-like para ERPs).
--
-- IDEMPOTENCIA: cada objeto se crea solo si no existe (guard en RDB$).
-- Patrón EXECUTE BLOCK + EXECUTE STATEMENT (compatible con el runner que
-- divide por ';' y no soporta bloques PSQL con ';' internos).

-- ── Excepciones nombradas (Firebird EXCEPTION requiere un identificador) ──

EXECUTE BLOCK
AS BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$EXCEPTIONS WHERE RDB$EXCEPTION_NAME = 'EXC_AUDIT_NO_UPDATE')) THEN
    EXECUTE STATEMENT 'CREATE EXCEPTION EXC_AUDIT_NO_UPDATE ''Auditoria es append-only: no se puede modificar''';
END;

EXECUTE BLOCK
AS BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$EXCEPTIONS WHERE RDB$EXCEPTION_NAME = 'EXC_AUDIT_NO_DELETE')) THEN
    EXECUTE STATEMENT 'CREATE EXCEPTION EXC_AUDIT_NO_DELETE ''Auditoria es append-only: no se puede eliminar''';
END;

-- ── Trigger BEFORE UPDATE: bloquea cualquier UPDATE sobre auditoria ──

EXECUTE BLOCK
AS BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$TRIGGERS WHERE RDB$TRIGGER_NAME = 'TRG_AUDITORIA_NO_UPDATE')) THEN
    EXECUTE STATEMENT 'CREATE TRIGGER trg_auditoria_no_update FOR auditoria ACTIVE BEFORE UPDATE AS BEGIN EXCEPTION EXC_AUDIT_NO_UPDATE; END';
END;

-- ── Trigger BEFORE DELETE: bloquea cualquier DELETE sobre auditoria ──

EXECUTE BLOCK
AS BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$TRIGGERS WHERE RDB$TRIGGER_NAME = 'TRG_AUDITORIA_NO_DELETE')) THEN
    EXECUTE STATEMENT 'CREATE TRIGGER trg_auditoria_no_delete FOR auditoria ACTIVE BEFORE DELETE AS BEGIN EXCEPTION EXC_AUDIT_NO_DELETE; END';
END;

-- ── Comentario documental (sin efecto funcional) ──
-- Para mantenimiento legítimo (purgar auditoría antigua por retención):
--   1. Temporalmente desactivar: ALTER TRIGGER trg_auditoria_no_delete INACTIVE;
--   2. Hacer el DELETE con WHERE fecha < CAST('2020-01-01' AS DATE);
--   3. Reactivar: ALTER TRIGGER trg_auditoria_no_delete ACTIVE;
-- Esto deja rastro en RDB$TRIGGERS.RDB$TRIGGER_INACTIVE (timestamp de modificación)
-- y debería documentarse en un procedimiento operacional separado.
