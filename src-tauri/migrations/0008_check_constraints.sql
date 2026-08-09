-- 0008_check_constraints.sql
-- CHECK constraints para columnas tipo enum.
--
-- Valores verificados contra:
--   - data/config.ini (business.estados_*, business.tipos_*)
--   - services/*.rs (constantes hardcoded y validaciones)
--   - frontend: src/routes/*/+page.svelte (dropdowns y listas)
--   - 0001_initial_schema.sql (DEFAULTs de columna)
--
-- ADVERTENCIA: un ALTER TABLE ADD CONSTRAINT CHECK valida TODAS las filas
-- existentes. Si hay datos legacy con valores fuera de la lista, la migración
-- ABORTA. Verificado en la BD de desarrollo (data/dinamo_rent_v3.fdb):
--   rentas.estado = Activo | autos.estado = Baja, Disponible
--   clientes.estado = Activo | reservas/comparendos = sin filas
-- Antes de aplicar en producción, ejecutar:
--   SELECT DISTINCT estado FROM rentas;
--   SELECT DISTINCT estado FROM autos;
--   SELECT DISTINCT estado FROM clientes;
--   SELECT DISTINCT estado FROM reservas;
--   SELECT DISTINCT estado FROM comparendos;
-- y ajustar este archivo si aparecen valores no listados.
--
-- IDEMPOTENCIA: cada ADD CONSTRAINT va guardado contra RDB$RELATION_CONSTRAINTS
-- (EXECUTE BLOCK + EXECUTE STATEMENT). Si la constraint ya existe (BD a medias
-- de una ejecución anterior), se omite y la migración continúa.

-- rentas.estado: valores verificados en repositories/renta.rs (insert='Activo',
-- cerrar='Cerrada', cancelar='Cancelada') + activas() filtra 'Activa' OR 'Activo'
-- (legacy). NO es config-driven (hardcoded en Rust).
EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$RELATION_CONSTRAINTS WHERE RDB$CONSTRAINT_NAME = 'CHK_RENTAS_ESTADO')) THEN
    EXECUTE STATEMENT 'ALTER TABLE rentas ADD CONSTRAINT chk_rentas_estado CHECK (estado IN (''Activo'', ''Activa'', ''Cerrada'', ''Cancelada''))';
END;

-- autos.estado: verificado en config.ini (business.estados_auto) y frontend.
-- Si customiza config.ini, ajuste este constraint.
EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$RELATION_CONSTRAINTS WHERE RDB$CONSTRAINT_NAME = 'CHK_AUTOS_ESTADO')) THEN
    EXECUTE STATEMENT 'ALTER TABLE autos ADD CONSTRAINT chk_autos_estado CHECK (estado IN (''Disponible'', ''Rentado'', ''Mantenimiento'', ''Vendido'', ''Baja''))';
END;

-- clientes.estado: verificado en config.ini (business.estados_cliente) y frontend.
EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$RELATION_CONSTRAINTS WHERE RDB$CONSTRAINT_NAME = 'CHK_CLIENTES_ESTADO')) THEN
    EXECUTE STATEMENT 'ALTER TABLE clientes ADD CONSTRAINT chk_clientes_estado CHECK (estado IN (''Activo'', ''Inactivo'', ''Lista Negra'', ''VIP''))';
END;

-- reservas.estado: verificado en config.ini (business.estados_reserva) y frontend.
EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$RELATION_CONSTRAINTS WHERE RDB$CONSTRAINT_NAME = 'CHK_RESERVAS_ESTADO')) THEN
    EXECUTE STATEMENT 'ALTER TABLE reservas ADD CONSTRAINT chk_reservas_estado CHECK (estado IN (''Pendiente'', ''Confirmada'', ''Cancelada'', ''Completada''))';
END;

-- comparendos.estado: hardcoded en services/comparendo.rs (ESTADOS_COMPARENDO).
EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$RELATION_CONSTRAINTS WHERE RDB$CONSTRAINT_NAME = 'CHK_COMPARENDOS_ESTADO')) THEN
    EXECUTE STATEMENT 'ALTER TABLE comparendos ADD CONSTRAINT chk_comparendos_estado CHECK (estado IN (''Pendiente'', ''Pagado''))';
END;

-- TODO: verificar valores con el equipo.
-- pagos.metodo_pago: valores del dropdown en src/routes/rentas/+page.svelte:
--   'Efectivo', 'Tarjeta debito', 'Tarjeta credito', 'Transferencia',
--   'Nequi', 'Daviplata', 'Otro'.
-- NO se añade CHECK porque el service (services/renta.rs::registrar_pago) NO
-- valida el valor contra una lista (solo XSS + longitud), así que podría haber
-- datos legacy con otros valores. Antes de añadirlo, ejecutar:
--   SELECT DISTINCT metodo_pago FROM pagos;
-- y descomentar (notar: los valores llevan tilde en la BD real):
-- EXECUTE BLOCK
-- AS
-- BEGIN
--   IF (NOT EXISTS(SELECT 1 FROM RDB$RELATION_CONSTRAINTS WHERE RDB$CONSTRAINT_NAME = 'CHK_PAGOS_METODO_PAGO')) THEN
--     EXECUTE STATEMENT 'ALTER TABLE pagos ADD CONSTRAINT chk_pagos_metodo_pago CHECK (metodo_pago IN (''Efectivo'', ''Tarjeta débito'', ''Tarjeta crédito'', ''Transferencia'', ''Nequi'', ''Daviplata'', ''Otro''))';
-- END;

-- TODO: verificar valores con el equipo.
-- gastos.categoria: valores en config.ini (business.tipos_gasto):
--   'Combustible', 'Peajes', 'Lavado', 'Mantenimiento', 'Repuestos',
--   'Parqueadero', 'Seguros', 'Multas', 'Papelería', 'Otros'.
-- NO se añade CHECK porque la lista es config-driven (si un deployment
-- customiza config.ini, el CHECK hardcoded rompería inserts válidos). El
-- service (services/gasto.rs::validar) ya valida contra config.ini en runtime.
