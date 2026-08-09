-- 0005_tema_usuario.sql
-- Preferencia de tema por usuario: 'light' | 'dark' | 'auto'.
-- NULL = el usuario nunca la configuró (el frontend usa su valor por defecto).

ALTER TABLE usuarios ADD tema VARCHAR(10);
