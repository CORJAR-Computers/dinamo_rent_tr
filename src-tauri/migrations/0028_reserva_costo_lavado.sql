-- 0028: Agregar campo costo_lavado a reservas
-- Permite registrar el costo de lavado al momento de crear la reserva,
-- y se hereda al convertirla en renta.
SET TERM ^;
EXECUTE BLOCK AS BEGIN
    IF (NOT EXISTS (
        SELECT 1 FROM RDB$RELATION_FIELDS
        WHERE RDB$RELATION_NAME = 'RESERVAS'
          AND RDB$FIELD_NAME = 'COSTO_LAVADO'
    )) THEN BEGIN
        EXECUTE STATEMENT
            'ALTER TABLE reservas ADD costo_lavado DECIMAL(12,2) DEFAULT 0.00 NOT NULL';
    END
END^
SET TERM ;
