-- 0007_triggers_updated_at.sql
-- Triggers BEFORE UPDATE que setean updated_at = CURRENT_TIMESTAMP.
--
-- IMPORTANTE (limitación del runner de migraciones):
-- El runner (core/migrations.rs) divide el SQL por ';' tras retirar las
-- líneas de comentario completas. NO soporta bloques PSQL con ';' internos
-- (EXECUTE BLOCK, triggers con varias sentencias). Por eso cada trigger se
-- escribe con UNA SOLA sentencia dentro del BEGIN...END y SIN ';' interno.
-- El único ';' es el que termina el CREATE TRIGGER (después de END).
--
-- Firebird acepta un BEGIN...END con una única sentencia y sin separador ';'
-- antes del END (el ';' es separador entre sentencias, no terminador). Ver
-- gramática PSQL de Firebird 5.0 y resources/firebird/doc/sql.extensions/
-- README.execute_block.txt (los ejemplos omiten el ';' tras la última
-- sentencia antes del END).
--
-- Comportamiento: el trigger setea SIEMPRE updated_at en cada UPDATE (no
-- solo si es NULL). Esto es intencional para columnas de auditoría:
-- updated_at debe reflejar el último UPDATE. Todos los repositories ya
-- hacen updated_at = CURRENT_TIMESTAMP explícito, así que el trigger actúa
-- como red de seguridad (y cubre UPDATEs que se olviden de setearlo).
--
-- Tablas con updated_at (verificado en 0001_initial_schema.sql):
--   usuarios, autos, clientes, reservas, mantenimiento_vehiculos,
--   gastos, comparendos, pagos.
-- rentas NO tenía updated_at (solo created_at): se añade primero.
--
-- IDEMPOTENCIA (importante):
--   - El ALTER de rentas.updated_at va guardado contra RDB$RELATION_FIELDS.
--   - Los triggers usan RECREATE TRIGGER (crea si no existe, recrea si existe).
-- Esto hace la migración auto-reparable: una BD que quedó a medias de una
-- ejecución anterior (columna creada pero migración sin registrar y triggers
-- sin crear) se repara sola en el siguiente arranque.

EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$RELATION_FIELDS WHERE RDB$RELATION_NAME = 'RENTAS' AND RDB$FIELD_NAME = 'UPDATED_AT')) THEN
    EXECUTE STATEMENT 'ALTER TABLE rentas ADD updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL';
END;

RECREATE TRIGGER trg_usuarios_updated_at FOR usuarios
ACTIVE BEFORE UPDATE POSITION 0
AS
BEGIN
  NEW.updated_at = CURRENT_TIMESTAMP;
END;

RECREATE TRIGGER trg_autos_updated_at FOR autos
ACTIVE BEFORE UPDATE POSITION 0
AS
BEGIN
  NEW.updated_at = CURRENT_TIMESTAMP;
END;

RECREATE TRIGGER trg_clientes_updated_at FOR clientes
ACTIVE BEFORE UPDATE POSITION 0
AS
BEGIN
  NEW.updated_at = CURRENT_TIMESTAMP;
END;

RECREATE TRIGGER trg_reservas_updated_at FOR reservas
ACTIVE BEFORE UPDATE POSITION 0
AS
BEGIN
  NEW.updated_at = CURRENT_TIMESTAMP;
END;

RECREATE TRIGGER trg_rentas_updated_at FOR rentas
ACTIVE BEFORE UPDATE POSITION 0
AS
BEGIN
  NEW.updated_at = CURRENT_TIMESTAMP;
END;

RECREATE TRIGGER trg_mantenimiento_vehiculos_updated_at FOR mantenimiento_vehiculos
ACTIVE BEFORE UPDATE POSITION 0
AS
BEGIN
  NEW.updated_at = CURRENT_TIMESTAMP;
END;

RECREATE TRIGGER trg_gastos_updated_at FOR gastos
ACTIVE BEFORE UPDATE POSITION 0
AS
BEGIN
  NEW.updated_at = CURRENT_TIMESTAMP;
END;

RECREATE TRIGGER trg_comparendos_updated_at FOR comparendos
ACTIVE BEFORE UPDATE POSITION 0
AS
BEGIN
  NEW.updated_at = CURRENT_TIMESTAMP;
END;

RECREATE TRIGGER trg_pagos_updated_at FOR pagos
ACTIVE BEFORE UPDATE POSITION 0
AS
BEGIN
  NEW.updated_at = CURRENT_TIMESTAMP;
END;
