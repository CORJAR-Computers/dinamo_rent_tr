-- 0018_empresa_ciudad.sql — Columna CIUDAD en EMPRESA_CONFIG
--
-- El setup inicial (/empresa) recogía nombre, NIT, dirección, teléfono,
-- email, web y logo, pero la CIUDAD se derivaba de la dirección con una
-- heurística (la penúltima parte entre comas) — frágil y confusa. Esta
-- migración añade la columna explícita para que cada empresa compradora
-- configure su ciudad y los contratos/órdenes la usen directamente.
--
-- IDEMPOTENCIA (mismo patrón que 0015): el guard revisa RDB$RELATION_FIELDS
-- antes del DDL, así que reintentos e instalaciones nuevas son seguros.

EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$RELATION_FIELDS rf
                 WHERE rf.RDB$RELATION_NAME = 'EMPRESA_CONFIG'
                   AND rf.RDB$FIELD_NAME = 'CIUDAD')) THEN
    EXECUTE STATEMENT 'ALTER TABLE EMPRESA_CONFIG ADD CIUDAD VARCHAR(100)';
END;
