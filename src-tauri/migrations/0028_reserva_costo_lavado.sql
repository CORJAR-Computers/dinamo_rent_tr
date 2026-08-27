-- 0028: Agregar campo costo_lavado a reservas
-- Permite registrar el costo de lavado al momento de crear la reserva,
-- y se hereda al convertirla en renta.
--
-- NOTA: el runner de migraciones (core/migrations.rs) divide por ';' y
-- maneja bloques BEGIN...END, pero NO soporta SET TERM. Los bloques
-- EXECUTE BLOCK se escriben sin SET TERM para compatibilidad.
EXECUTE BLOCK AS BEGIN
    IF (NOT EXISTS (
        SELECT 1 FROM RDB$RELATION_FIELDS
        WHERE RDB$RELATION_NAME = 'RESERVAS'
          AND RDB$FIELD_NAME = 'COSTO_LAVADO'
    )) THEN BEGIN
        EXECUTE STATEMENT
            'ALTER TABLE reservas ADD costo_lavado DECIMAL(12,2) DEFAULT 0.00 NOT NULL';
    END
END
