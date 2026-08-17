-- 0023_renta_comision.sql — Comisión por renta (checkbox + valor)
--
-- La empresa puede pagar comisión a intermediarios/agencias por una renta.
-- El operador marca un checkbox (tiene_comision) y digita el valor; la
-- comisión se resta del total para obtener el valor neto (información
-- financiera), SIN cambiar lo que cobra al cliente (total/saldo se mantienen).
--
-- DEFAULT 0: las rentas existentes no tienen comisión; las nuevas envían el
-- valor explícito desde el formulario.
--
-- IDEMPOTENCIA (mismo patrón que 0019/0021): el guard revisa
-- RDB$RELATION_FIELDS antes del DDL, así que reintentos e instalaciones
-- nuevas son seguros.

EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$RELATION_FIELDS rf
                 WHERE rf.RDB$RELATION_NAME = 'RENTAS'
                   AND rf.RDB$FIELD_NAME = 'TIENE_COMISION')) THEN
    EXECUTE STATEMENT 'ALTER TABLE RENTAS ADD TIENE_COMISION SMALLINT DEFAULT 0 NOT NULL';
  IF (NOT EXISTS(SELECT 1 FROM RDB$RELATION_FIELDS rf
                 WHERE rf.RDB$RELATION_NAME = 'RENTAS'
                   AND rf.RDB$FIELD_NAME = 'COMISION')) THEN
    EXECUTE STATEMENT 'ALTER TABLE RENTAS ADD COMISION DECIMAL(12,2) DEFAULT 0 NOT NULL';
  IF (NOT EXISTS(SELECT 1 FROM RDB$RELATION_FIELDS rf
                 WHERE rf.RDB$RELATION_NAME = 'RENTAS'
                   AND rf.RDB$FIELD_NAME = 'VALOR_NETO')) THEN
    EXECUTE STATEMENT 'ALTER TABLE RENTAS ADD VALOR_NETO DECIMAL(12,2) DEFAULT 0 NOT NULL';
END;

-- Backfill: las rentas anteriores a esta migración no tienen comisión, así
-- que su valor neto = total (no 0). Idempotente: solo toca rentas sin
-- comisión; con comisión el servicio ya calculó neto = total − comisión.
UPDATE rentas SET valor_neto = total WHERE comision = 0 AND total > 0;
