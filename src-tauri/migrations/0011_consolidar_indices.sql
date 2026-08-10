-- 0011_consolidar_indices.sql
-- Consolidación de índices redundantes detectados en la auditoría de la BD dev:
--
-- A) IX_RENTAS_ESTADO — legacy: ninguna migración actual lo crea (residuo del
--    0001 antiguo); duplica IDX_RENTAS_ESTADO (0001). Solo existe en BDs viejas.
--
-- B) RENTAS(placa) — alinear la BD dev con las instalaciones nuevas: crear
--    IDX_RENTAS_PLACA (el canónico de 0002, el que 0010 conservó) y eliminar el
--    IX_RENTAS_PLACA de 0001 (que 0010 no pudo dropear por no existir el IDX_).
--
-- C) Índices manuales redundantes con el índice automático de la FK — existen
--    en TODAS las instalaciones; el planner usa el de la FK (verificado con
--    SET PLAN ON: PLAN (RENTAS INDEX (RDB$FOREIGN38)), etc.):
--      - IX_RENTAS_ID_CLIENTE     (0001) ← FK rentas.id_cliente  → RDB$FOREIGN39
--      - IX_GASTOS_PLACA          (0001) ← FK gastos.placa       → RDB$FOREIGN36
--      - IX_INSPECCIONES_ID_RENTA (0009) ← FK inspecciones.id_renta → RDB$FOREIGN42
--      - IDX_MANTENIMIENTO_PLACA  (0002) ← FK mantenimiento.placa → RDB$FOREIGN34
--    (0009 creó el de inspecciones creyendo que no había índice sobre id_renta,
--    pero pasó por alto el índice automático de la FK.)
--
-- 0001/0002/0009 dejan de crear estos índices (solo afecta a instalaciones
-- nuevas); aquí se eliminan de BDs existentes.
--
-- IDEMPOTENCIA/SEGURIDAD: cada DROP solo se ejecuta si queda OTRO índice
-- (no sistema) que cubra la misma columna — nunca se deja una columna de
-- búsqueda sin índice. Re-ejecuciones y BDs ya consolidadas son no-op.

-- B1: crear el canónico de rentas(placa) si falta (alineación dev ↔ fresh)
EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$INDICES WHERE RDB$INDEX_NAME = 'IDX_RENTAS_PLACA')) THEN
    EXECUTE STATEMENT 'CREATE INDEX IDX_RENTAS_PLACA ON RENTAS (placa)';
END;

-- B2: eliminar el IX_ de 0001 (el IDX_ creado o el índice de la FK siguen
-- cubriendo la columna PLACA).
EXECUTE BLOCK
AS
BEGIN
  IF (EXISTS(SELECT 1 FROM RDB$INDICES WHERE RDB$INDEX_NAME = 'IX_RENTAS_PLACA')
      AND EXISTS(SELECT 1 FROM RDB$INDEX_SEGMENTS s
                  JOIN RDB$INDICES i2 ON i2.RDB$INDEX_NAME = s.RDB$INDEX_NAME
                 WHERE i2.RDB$SYSTEM_FLAG = 0
                   AND i2.RDB$RELATION_NAME = 'RENTAS'
                   AND i2.RDB$INDEX_NAME <> 'IX_RENTAS_PLACA'
                   AND s.RDB$FIELD_NAME = 'PLACA')) THEN
    EXECUTE STATEMENT 'DROP INDEX IX_RENTAS_PLACA';
END;

-- A: legacy rentas(estado) — IDX_RENTAS_ESTADO (y los compuestos de estado)
-- siguen cubriendo ESTADO.
EXECUTE BLOCK
AS
BEGIN
  IF (EXISTS(SELECT 1 FROM RDB$INDICES WHERE RDB$INDEX_NAME = 'IX_RENTAS_ESTADO')
      AND EXISTS(SELECT 1 FROM RDB$INDEX_SEGMENTS s
                  JOIN RDB$INDICES i2 ON i2.RDB$INDEX_NAME = s.RDB$INDEX_NAME
                 WHERE i2.RDB$SYSTEM_FLAG = 0
                   AND i2.RDB$RELATION_NAME = 'RENTAS'
                   AND i2.RDB$INDEX_NAME <> 'IX_RENTAS_ESTADO'
                   AND s.RDB$FIELD_NAME = 'ESTADO')) THEN
    EXECUTE STATEMENT 'DROP INDEX IX_RENTAS_ESTADO';
END;

-- C1: rentas(id_cliente) ← índice de la FK
EXECUTE BLOCK
AS
BEGIN
  IF (EXISTS(SELECT 1 FROM RDB$INDICES WHERE RDB$INDEX_NAME = 'IX_RENTAS_ID_CLIENTE')
      AND EXISTS(SELECT 1 FROM RDB$INDEX_SEGMENTS s
                  JOIN RDB$INDICES i2 ON i2.RDB$INDEX_NAME = s.RDB$INDEX_NAME
                 WHERE i2.RDB$SYSTEM_FLAG = 0
                   AND i2.RDB$RELATION_NAME = 'RENTAS'
                   AND i2.RDB$INDEX_NAME <> 'IX_RENTAS_ID_CLIENTE'
                   AND s.RDB$FIELD_NAME = 'ID_CLIENTE')) THEN
    EXECUTE STATEMENT 'DROP INDEX IX_RENTAS_ID_CLIENTE';
END;

-- C2: gastos(placa) ← índice de la FK
EXECUTE BLOCK
AS
BEGIN
  IF (EXISTS(SELECT 1 FROM RDB$INDICES WHERE RDB$INDEX_NAME = 'IX_GASTOS_PLACA')
      AND EXISTS(SELECT 1 FROM RDB$INDEX_SEGMENTS s
                  JOIN RDB$INDICES i2 ON i2.RDB$INDEX_NAME = s.RDB$INDEX_NAME
                 WHERE i2.RDB$SYSTEM_FLAG = 0
                   AND i2.RDB$RELATION_NAME = 'GASTOS'
                   AND i2.RDB$INDEX_NAME <> 'IX_GASTOS_PLACA'
                   AND s.RDB$FIELD_NAME = 'PLACA')) THEN
    EXECUTE STATEMENT 'DROP INDEX IX_GASTOS_PLACA';
END;

-- C3: inspecciones(id_renta) ← índice de la FK
EXECUTE BLOCK
AS
BEGIN
  IF (EXISTS(SELECT 1 FROM RDB$INDICES WHERE RDB$INDEX_NAME = 'IX_INSPECCIONES_ID_RENTA')
      AND EXISTS(SELECT 1 FROM RDB$INDEX_SEGMENTS s
                  JOIN RDB$INDICES i2 ON i2.RDB$INDEX_NAME = s.RDB$INDEX_NAME
                 WHERE i2.RDB$SYSTEM_FLAG = 0
                   AND i2.RDB$RELATION_NAME = 'INSPECCIONES'
                   AND i2.RDB$INDEX_NAME <> 'IX_INSPECCIONES_ID_RENTA'
                   AND s.RDB$FIELD_NAME = 'ID_RENTA')) THEN
    EXECUTE STATEMENT 'DROP INDEX IX_INSPECCIONES_ID_RENTA';
END;

-- C4a: mantenimiento(placa) — IX_ de 0001 (lo cubren el IDX_ o la FK)
EXECUTE BLOCK
AS
BEGIN
  IF (EXISTS(SELECT 1 FROM RDB$INDICES WHERE RDB$INDEX_NAME = 'IX_MANTENIMIENTO_VEHICULOS_PLACA')
      AND EXISTS(SELECT 1 FROM RDB$INDEX_SEGMENTS s
                  JOIN RDB$INDICES i2 ON i2.RDB$INDEX_NAME = s.RDB$INDEX_NAME
                 WHERE i2.RDB$SYSTEM_FLAG = 0
                   AND i2.RDB$RELATION_NAME = 'MANTENIMIENTO_VEHICULOS'
                   AND i2.RDB$INDEX_NAME <> 'IX_MANTENIMIENTO_VEHICULOS_PLACA'
                   AND s.RDB$FIELD_NAME = 'PLACA')) THEN
    EXECUTE STATEMENT 'DROP INDEX IX_MANTENIMIENTO_VEHICULOS_PLACA';
END;

-- C4b: mantenimiento(placa) — IDX_ de 0002 (lo cubre el índice de la FK)
EXECUTE BLOCK
AS
BEGIN
  IF (EXISTS(SELECT 1 FROM RDB$INDICES WHERE RDB$INDEX_NAME = 'IDX_MANTENIMIENTO_PLACA')
      AND EXISTS(SELECT 1 FROM RDB$INDEX_SEGMENTS s
                  JOIN RDB$INDICES i2 ON i2.RDB$INDEX_NAME = s.RDB$INDEX_NAME
                 WHERE i2.RDB$SYSTEM_FLAG = 0
                   AND i2.RDB$RELATION_NAME = 'MANTENIMIENTO_VEHICULOS'
                   AND i2.RDB$INDEX_NAME <> 'IDX_MANTENIMIENTO_PLACA'
                   AND s.RDB$FIELD_NAME = 'PLACA')) THEN
    EXECUTE STATEMENT 'DROP INDEX IDX_MANTENIMIENTO_PLACA';
END;
