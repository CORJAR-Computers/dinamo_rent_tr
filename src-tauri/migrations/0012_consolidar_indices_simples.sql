-- 0012_consolidar_indices_simples.sql
-- Elimina los índices de UNA columna que quedan subsumidos por un índice
-- compuesto cuyo PRIMER segmento es esa misma columna (prefijo izquierdo):
-- Firebird puede usar el compuesto para cualquier consulta sobre el prefijo,
-- así que el índice estrecho solo añade coste de escritura sin cobertura
-- nueva. Corresponde a la "observación secundaria" de la auditoría de índices
-- (la única pendiente: IX_GASTOS_PLACA ← IX_GASTOS_PLACA_FECHA ya se resolvió
-- en 0011):
--   - IX_AUTOS_ESTADO     (0001) ← IX_AUTOS_ESTADO_TIPO     (0001: estado, tipo)
--   - IX_CLIENTES_ESTADO  (0001) ← IX_CLIENTES_ESTADO_NOMBRE (0001: estado, nombre_completo)
--   - IDX_RENTAS_ESTADO   (0001) ← IDX_RENTAS_ESTADO_FECHA_RETORNO / IDX_RENTAS_ESTADO_PLACA
--   - IX_RESERVAS_ESTADO  (0001) ← IX_RESERVAS_ESTADO_FECHA  (0001: estado, fecha_recogida)
--
-- 0001 deja de crearlos (solo afecta a instalaciones nuevas); aquí se
-- eliminan de las BDs existentes.
--
-- IDEMPOTENCIA/SEGURIDAD: cada DROP solo se ejecuta si queda OTRO índice
-- (no sistema) de la MISMA tabla que tenga la columna como PRIMER segmento
-- (RDB$FIELD_POSITION = 0) y ACTIVO (RDB$INDEX_INACTIVE = 0): nunca se deja
-- una columna sin cobertura de prefijo izquierdo usable. (Más estricto que
-- 0011: aquí la cobertura son compuestos que teóricamente podrían estar
-- inactivos.) Re-ejecuciones y BDs ya consolidadas son no-op.

-- AUTOS(estado) ← IX_AUTOS_ESTADO_TIPO
EXECUTE BLOCK
AS
BEGIN
  IF (EXISTS(SELECT 1 FROM RDB$INDICES WHERE RDB$INDEX_NAME = 'IX_AUTOS_ESTADO')
      AND EXISTS(SELECT 1 FROM RDB$INDEX_SEGMENTS s
                  JOIN RDB$INDICES i2 ON i2.RDB$INDEX_NAME = s.RDB$INDEX_NAME
                 WHERE i2.RDB$SYSTEM_FLAG = 0
                   AND i2.RDB$RELATION_NAME = 'AUTOS'
                   AND i2.RDB$INDEX_NAME <> 'IX_AUTOS_ESTADO'
                   AND s.RDB$FIELD_NAME = 'ESTADO'
                   AND s.RDB$FIELD_POSITION = 0
                   AND COALESCE(i2.RDB$INDEX_INACTIVE, 0) = 0)) THEN
    EXECUTE STATEMENT 'DROP INDEX IX_AUTOS_ESTADO';
END;

-- CLIENTES(estado) ← IX_CLIENTES_ESTADO_NOMBRE
EXECUTE BLOCK
AS
BEGIN
  IF (EXISTS(SELECT 1 FROM RDB$INDICES WHERE RDB$INDEX_NAME = 'IX_CLIENTES_ESTADO')
      AND EXISTS(SELECT 1 FROM RDB$INDEX_SEGMENTS s
                  JOIN RDB$INDICES i2 ON i2.RDB$INDEX_NAME = s.RDB$INDEX_NAME
                 WHERE i2.RDB$SYSTEM_FLAG = 0
                   AND i2.RDB$RELATION_NAME = 'CLIENTES'
                   AND i2.RDB$INDEX_NAME <> 'IX_CLIENTES_ESTADO'
                   AND s.RDB$FIELD_NAME = 'ESTADO'
                   AND s.RDB$FIELD_POSITION = 0
                   AND COALESCE(i2.RDB$INDEX_INACTIVE, 0) = 0)) THEN
    EXECUTE STATEMENT 'DROP INDEX IX_CLIENTES_ESTADO';
END;

-- RENTAS(estado) ← IDX_RENTAS_ESTADO_FECHA_RETORNO / IDX_RENTAS_ESTADO_PLACA
EXECUTE BLOCK
AS
BEGIN
  IF (EXISTS(SELECT 1 FROM RDB$INDICES WHERE RDB$INDEX_NAME = 'IDX_RENTAS_ESTADO')
      AND EXISTS(SELECT 1 FROM RDB$INDEX_SEGMENTS s
                  JOIN RDB$INDICES i2 ON i2.RDB$INDEX_NAME = s.RDB$INDEX_NAME
                 WHERE i2.RDB$SYSTEM_FLAG = 0
                   AND i2.RDB$RELATION_NAME = 'RENTAS'
                   AND i2.RDB$INDEX_NAME <> 'IDX_RENTAS_ESTADO'
                   AND s.RDB$FIELD_NAME = 'ESTADO'
                   AND s.RDB$FIELD_POSITION = 0
                   AND COALESCE(i2.RDB$INDEX_INACTIVE, 0) = 0)) THEN
    EXECUTE STATEMENT 'DROP INDEX IDX_RENTAS_ESTADO';
END;

-- RESERVAS(estado) ← IX_RESERVAS_ESTADO_FECHA
EXECUTE BLOCK
AS
BEGIN
  IF (EXISTS(SELECT 1 FROM RDB$INDICES WHERE RDB$INDEX_NAME = 'IX_RESERVAS_ESTADO')
      AND EXISTS(SELECT 1 FROM RDB$INDEX_SEGMENTS s
                  JOIN RDB$INDICES i2 ON i2.RDB$INDEX_NAME = s.RDB$INDEX_NAME
                 WHERE i2.RDB$SYSTEM_FLAG = 0
                   AND i2.RDB$RELATION_NAME = 'RESERVAS'
                   AND i2.RDB$INDEX_NAME <> 'IX_RESERVAS_ESTADO'
                   AND s.RDB$FIELD_NAME = 'ESTADO'
                   AND s.RDB$FIELD_POSITION = 0
                   AND COALESCE(i2.RDB$INDEX_INACTIVE, 0) = 0)) THEN
    EXECUTE STATEMENT 'DROP INDEX IX_RESERVAS_ESTADO';
END;
