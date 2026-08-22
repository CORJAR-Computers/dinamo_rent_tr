-- 0026: Agregar bandera "cobrar horas extras" a rentas.
-- Por defecto TRUE (compatibilidad con rentas existentes que ya cobraban HE).
-- Cuando el operador desmarca el checkbox, las horas extras no se cobran
-- aunque haya excedente de tiempo (cortesía al cliente).

SET TERM ^;

EXECUTE BLOCK AS
BEGIN
    -- Solo agregar la columna si no existe
    IF (NOT EXISTS (
        SELECT 1 FROM RDB$RELATION_FIELDS
        WHERE RDB$RELATION_NAME = 'RENTAS'
          AND RDB$FIELD_NAME = 'COBRAR_HORAS_EXTRA'
    )) THEN
    BEGIN
        ALTER TABLE RENTAS ADD COBRAR_HORAS_EXTRA DMN_BOOLEANO DEFAULT 1;
        UPDATE RENTAS SET COBRAR_HORAS_EXTRA = 1 WHERE COBRAR_HORAS_EXTRA IS NULL;
    END
END^

SET TERM ;^
