-- 0022_agente_simit_ultimo_resultado.sql
-- Persistencia del último resultado de sincronización del Agente SIMIT
-- (JSON) para que el filtro «Solo nuevos» y el panel de Comparendos
-- sobrevivan al reinicio de la app: al arrancar se restaura la última
-- corrida desde la BD en vez de partir en blanco.
--
-- Una sola fila (id = 1) que se sobrescribe en cada sincronización (upsert).
-- `resultado_json` es el `ResultadoSincronizacion` serializado (incluye los
-- registros con su id de `comparendos` → el frontend marca los 🆕 de la
-- última corrida tras un reinicio).
--
-- IDEMPOTENCIA (mismo patrón que 0001-0021): el guard revisa RDB$RELATIONS
-- antes del DDL.

EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$RELATIONS
                 WHERE RDB$RELATION_NAME = 'AGENTE_SIMIT_ULTIMO_RESULTADO')) THEN
    EXECUTE STATEMENT 'CREATE TABLE agente_simit_ultimo_resultado (
        id SMALLINT NOT NULL PRIMARY KEY,
        resultado_json BLOB SUB_TYPE TEXT NOT NULL,
        updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
    )';
END;
