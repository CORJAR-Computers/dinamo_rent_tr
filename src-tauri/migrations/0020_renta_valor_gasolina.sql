-- 0020_renta_valor_gasolina.sql — Cargo por gasolina en la renta
--
-- El formulario de renta permite ingresar el valor de la gasolina cuando el
-- usuario trae el auto sin tanquear o lo va a entregar sin tanquear. Ese valor
-- se suma a los costos extra al recalcular totales.
--
-- DEFAULT 0: las rentas existentes conservan sin cargo de gasolina.
--
-- IDEMPOTENCIA (mismo patrón que 0015/0018/0019): el guard revisa
-- RDB$RELATION_FIELDS antes del DDL, así que reintentos e instalaciones
-- nuevas son seguros.

EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$RELATION_FIELDS rf
                 WHERE rf.RDB$RELATION_NAME = 'RENTAS'
                   AND rf.RDB$FIELD_NAME = 'VALOR_GASOLINA')) THEN
    EXECUTE STATEMENT 'ALTER TABLE RENTAS ADD VALOR_GASOLINA DECIMAL(12,2) DEFAULT 0 NOT NULL';
END;
