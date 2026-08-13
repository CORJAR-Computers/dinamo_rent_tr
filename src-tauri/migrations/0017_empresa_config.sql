-- 0017_empresa_config.sql — Configuración de la empresa (setup inicial)
--
-- Tabla de UNA fila (ID = 1) con los datos que la empresa compradora
-- configura en el primer arranque: nombre, NIT, dirección, teléfono,
-- email, web y logo. El logo se guarda como ARCHIVO en <data_dir>/logos/
-- (el binario no viaja por Firebird) y aquí solo se persiste el nombre
-- del archivo (p. ej. 'empresa.png'); null = sin logo.
--
-- Idempotente: el guard contra RDB$RELATIONS evita duplicar la tabla si
-- la migración se reintenta (patrón 5.2 del Handsoff).

EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$RELATIONS WHERE RDB$RELATION_NAME = 'EMPRESA_CONFIG')) THEN
    EXECUTE STATEMENT '
      CREATE TABLE EMPRESA_CONFIG (
        ID          INT NOT NULL,
        NOMBRE      VARCHAR(120),
        NIT         VARCHAR(40),
        DIRECCION   VARCHAR(200),
        TELEFONO    VARCHAR(40),
        EMAIL       VARCHAR(120),
        WEB         VARCHAR(120),
        LOGO        VARCHAR(80),
        UPDATED_AT  TIMESTAMP,
        CONSTRAINT PK_EMPRESA_CONFIG PRIMARY KEY (ID)
      )';
END;
