ALTER TABLE rentas ADD anio_contrato SMALLINT;

UPDATE rentas SET anio_contrato = COALESCE(
    CAST(EXTRACT(YEAR FROM created_at) AS SMALLINT),
    CAST(EXTRACT(YEAR FROM fecha_recogida) AS SMALLINT)
);

UPDATE rentas r SET no_contrato = (
    SELECT seq FROM (
        SELECT id, ROW_NUMBER() OVER (PARTITION BY anio_contrato ORDER BY COALESCE(fecha_recogida, created_at), id) AS seq
        FROM rentas
    ) x WHERE x.id = r.id
);

DROP INDEX ix_rentas_no_contrato;

CREATE UNIQUE INDEX ix_rentas_no_contrato_anio ON rentas (anio_contrato, no_contrato);

ALTER TABLE rentas ALTER COLUMN anio_contrato SET NOT NULL;

ALTER TABLE rentas ALTER COLUMN no_contrato SET NOT NULL;
