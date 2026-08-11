-- 0016_atribucion_comparendo_renta.sql
-- Backfill de atribución de comparendos: vincula cada comparendo sin
-- renta/cliente con la renta que cubría el vehículo el día de la infracción
-- (id_renta / id_cliente), con la misma lógica que el cruce comparendos↔rentas
-- y la atribución automática del Agente SIMIT (renta_del_dia).
--
-- Es DML (no DDL) e idempotente: el WHERE ignora los comparendos ya atribuidos
-- y los que no tenían renta ese día (EXISTS), así que una re-ejecución (runner
-- en autocommit) no pisa ni duplica nada. Si hubiera solapamiento, ambas
-- subconsultas eligen la renta de recogida más reciente (FIRST 1 con ORDER BY
-- fecha_recogida DESC, id DESC) — misma renta en ambas.
UPDATE comparendos c
SET id_renta = (SELECT FIRST 1 r.id FROM rentas r
                WHERE r.placa = c.placa
                  AND r.fecha_recogida <= c.fecha_infraccion
                  AND COALESCE(r.fecha_devolucion_real, r.fecha_retorno) >= c.fecha_infraccion
                  AND r.deleted_at IS NULL AND r.estado <> 'Cancelada'
                ORDER BY r.fecha_recogida DESC, r.id DESC),
    id_cliente = (SELECT FIRST 1 r.id_cliente FROM rentas r
                  WHERE r.placa = c.placa
                    AND r.fecha_recogida <= c.fecha_infraccion
                    AND COALESCE(r.fecha_devolucion_real, r.fecha_retorno) >= c.fecha_infraccion
                    AND r.deleted_at IS NULL AND r.estado <> 'Cancelada'
                  ORDER BY r.fecha_recogida DESC, r.id DESC),
    updated_at = CURRENT_TIMESTAMP
WHERE c.deleted_at IS NULL
  AND c.id_renta IS NULL AND c.id_cliente IS NULL
  AND EXISTS (SELECT 1 FROM rentas r
              WHERE r.placa = c.placa
                AND r.fecha_recogida <= c.fecha_infraccion
                AND COALESCE(r.fecha_devolucion_real, r.fecha_retorno) >= c.fecha_infraccion
                AND r.deleted_at IS NULL AND r.estado <> 'Cancelada');
