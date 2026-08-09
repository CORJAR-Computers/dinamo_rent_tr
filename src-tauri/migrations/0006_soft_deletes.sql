-- 0006_soft_deletes.sql
-- Soft deletes en tablas financieras: añade deleted_at para trazabilidad.
-- Los registros NO se borran físicamente; se marcan con deleted_at y los
-- repositories los filtran con WHERE deleted_at IS NULL.
--
-- Tablas afectadas (verificado en 0001_initial_schema.sql):
--   rentas, pagos, gastos, comparendos, mantenimiento_vehiculos
--
-- Nota: inspecciones NO se incluye porque es una tabla satélite de rentas
-- (cae con la renta al hacer soft-delete en cascada desde el repository).
--
-- IDEMPOTENCIA (importante): cada ALTER/INDEX va envuelto en un EXECUTE BLOCK
-- que comprueba primero en el catálogo (RDB$RELATION_FIELDS / RDB$INDICES) si
-- el objeto ya existe. Si una ejecución anterior quedó a medias (columna o
-- índice creado pero migración no registrada en schema_migrations), esta
-- migración se auto-repara en el siguiente arranque en vez de abortar con
-- "violation of PRIMARY or UNIQUE KEY ... RDB$RELATION_FIELDS".
--
-- El runner (core/migrations.rs) divide el SQL por ';' respetando bloques
-- BEGIN...END, así que los EXECUTE BLOCK se ejecutan como una sola sentencia
-- (cada bloque termina con END;). DDL dentro de EXECUTE STATEMENT es válido en
-- Firebird cuando lo ejecuta un usuario con privilegios (SYSDBA/embedded).

EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$RELATION_FIELDS WHERE RDB$RELATION_NAME = 'RENTAS' AND RDB$FIELD_NAME = 'DELETED_AT')) THEN
    EXECUTE STATEMENT 'ALTER TABLE rentas ADD deleted_at TIMESTAMP';
END;

EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$RELATION_FIELDS WHERE RDB$RELATION_NAME = 'PAGOS' AND RDB$FIELD_NAME = 'DELETED_AT')) THEN
    EXECUTE STATEMENT 'ALTER TABLE pagos ADD deleted_at TIMESTAMP';
END;

EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$RELATION_FIELDS WHERE RDB$RELATION_NAME = 'GASTOS' AND RDB$FIELD_NAME = 'DELETED_AT')) THEN
    EXECUTE STATEMENT 'ALTER TABLE gastos ADD deleted_at TIMESTAMP';
END;

EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$RELATION_FIELDS WHERE RDB$RELATION_NAME = 'COMPARENDOS' AND RDB$FIELD_NAME = 'DELETED_AT')) THEN
    EXECUTE STATEMENT 'ALTER TABLE comparendos ADD deleted_at TIMESTAMP';
END;

EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$RELATION_FIELDS WHERE RDB$RELATION_NAME = 'MANTENIMIENTO_VEHICULOS' AND RDB$FIELD_NAME = 'DELETED_AT')) THEN
    EXECUTE STATEMENT 'ALTER TABLE mantenimiento_vehiculos ADD deleted_at TIMESTAMP';
END;

EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$INDICES WHERE RDB$INDEX_NAME = 'IX_RENTAS_DELETED')) THEN
    EXECUTE STATEMENT 'CREATE INDEX ix_rentas_deleted ON rentas (deleted_at)';
END;

EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$INDICES WHERE RDB$INDEX_NAME = 'IX_PAGOS_DELETED')) THEN
    EXECUTE STATEMENT 'CREATE INDEX ix_pagos_deleted ON pagos (deleted_at)';
END;

EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$INDICES WHERE RDB$INDEX_NAME = 'IX_GASTOS_DELETED')) THEN
    EXECUTE STATEMENT 'CREATE INDEX ix_gastos_deleted ON gastos (deleted_at)';
END;

EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$INDICES WHERE RDB$INDEX_NAME = 'IX_COMPARENDOS_DELETED')) THEN
    EXECUTE STATEMENT 'CREATE INDEX ix_comparendos_deleted ON comparendos (deleted_at)';
END;

EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$INDICES WHERE RDB$INDEX_NAME = 'IX_MANTENIMIENTO_DELETED')) THEN
    EXECUTE STATEMENT 'CREATE INDEX ix_mantenimiento_deleted ON mantenimiento_vehiculos (deleted_at)';
END;
