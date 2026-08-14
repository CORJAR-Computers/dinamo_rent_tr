-- 0019_renta_cobra_iva.sql — Flag COBRA_IVA por renta
--
-- El IVA se aplicaba siempre (business.impuesto_porcentaje de config.ini) al
-- crear/editar/cerrar una renta. Ahora el operador decide por renta con un
-- checkbox: cobra_iva = 1 aplica el porcentaje configurado, 0 no cobra IVA.
--
-- DEFAULT 1: las rentas existentes (creadas bajo el régimen de IVA automático)
-- conservan su comportamiento al cerrarse; las nuevas rentas envían el valor
-- explícito desde el formulario.
--
-- IDEMPOTENCIA (mismo patrón que 0015/0018): el guard revisa
-- RDB$RELATION_FIELDS antes del DDL, así que reintentos e instalaciones
-- nuevas son seguros.

EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$RELATION_FIELDS rf
                 WHERE rf.RDB$RELATION_NAME = 'RENTAS'
                   AND rf.RDB$FIELD_NAME = 'COBRA_IVA')) THEN
    EXECUTE STATEMENT 'ALTER TABLE RENTAS ADD COBRA_IVA SMALLINT DEFAULT 1 NOT NULL';
END;
