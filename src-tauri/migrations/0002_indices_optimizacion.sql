-- 0002_indices_optimizacion.sql
-- Índices para optimizar las consultas del informe mensual y búsquedas por placa

-- Optimización de búsquedas por placa
CREATE INDEX IDX_RENTAS_PLACA ON RENTAS (placa);
CREATE INDEX IDX_PAGOS_FECHA ON PAGOS (fecha);
CREATE INDEX IDX_MANTENIMIENTO_PLACA ON MANTENIMIENTO_VEHICULOS (placa);

-- Optimización de informe mensual (rango de fechas)
CREATE INDEX IDX_RENTAS_FECHA_RECOGIDA ON RENTAS (fecha_recogida);
CREATE INDEX IDX_MANTENIMIENTO_FECHA ON MANTENIMIENTO_VEHICULOS (pieza_varias_fecha);
CREATE INDEX IDX_COMPARENDOS_FECHA ON COMPARENDOS (fecha_infraccion);
CREATE INDEX IDX_GASTOS_FECHA ON GASTOS (fecha);
