-- 0005_tema_usuario.sql
-- Preferencia de tema por usuario: 'light' | 'dark' | 'auto'.
-- NULL = el usuario nunca la configuró (el frontend usa su valor por defecto).
--
-- IDEMPOTENCIA: el ALTER va guardado contra RDB$RELATION_FIELDS para
-- auto-reparar una BD que quedó con la columna creada pero la migración sin
-- registrar (estado parcial que antes rompía el arranque).

EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$RELATION_FIELDS WHERE RDB$RELATION_NAME = 'USUARIOS' AND RDB$FIELD_NAME = 'TEMA')) THEN
    EXECUTE STATEMENT 'ALTER TABLE usuarios ADD tema VARCHAR(10)';
END;
