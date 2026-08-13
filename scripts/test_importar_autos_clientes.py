#!/usr/bin/env python3
"""test_importar_autos_clientes.py — Tests unitarios de scripts/importar_autos_clientes.py.

Sin BD: cubren el parser (parse_sql_value / parse_sql_inserts), con el caso de
regresión del no_doc numérico que fromisoformat compacto (Python 3.11+)
corrompía como fecha.

Uso:
  python scripts/test_importar_autos_clientes.py
  python -m unittest scripts.test_importar_autos_clientes  (con scripts/ en path)
"""
import datetime
import importlib.util
import os
import sys
import unittest

_HERE = os.path.dirname(os.path.abspath(__file__))
_IMPORTER = os.path.join(_HERE, "importar_autos_clientes.py")

_spec = importlib.util.spec_from_file_location("imp_test", _IMPORTER)
imp = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(imp)


class TestParseSqlValue(unittest.TestCase):
    def test_no_doc_numerico_10_digitos_sigue_siendo_string(self):
        """REGRESIÓN: '1052070892' se corrompía a date(1052,7,8) (fromisoformat
        compacto en Python 3.11+/3.14). Debe seguir siendo el string."""
        v = imp.parse_sql_value("'1052070892'")
        self.assertEqual(v, "1052070892")
        self.assertIsInstance(v, str)

    def test_no_doc_numerico_8_digitos_sigue_siendo_string(self):
        self.assertEqual(imp.parse_sql_value("'10520708'"), "10520708")
        self.assertIsInstance(imp.parse_sql_value("'10520708'"), str)

    def test_no_doc_9_digitos_sigue_siendo_string(self):
        self.assertEqual(imp.parse_sql_value("'119431212'"), "119431212")

    def test_no_doc_con_letras_sigue_siendo_string(self):
        self.assertEqual(imp.parse_sql_value("'A22695624'"), "A22695624")
        self.assertEqual(imp.parse_sql_value("'C6K9GH4W1'"), "C6K9GH4W1")

    def test_fecha_iso_estricta_si_se_convierte(self):
        self.assertEqual(imp.parse_sql_value("'2034-02-26'"),
                         datetime.date(2034, 2, 26))

    def test_fecha_invalida_con_guiones_no_crashea(self):
        self.assertEqual(imp.parse_sql_value("'2024-13-99'"), "2024-13-99")

    def test_fecha_con_espacio_guiones_sigue_siendo_string(self):
        # Guiones mal ubicados: no es una fecha ISO estricta.
        self.assertEqual(imp.parse_sql_value("'2034-02'"), "2034-02")

    def test_datetime_con_hora_si_se_convierte(self):
        self.assertEqual(imp.parse_sql_value("'2034-02-26 10:30:00'"),
                         datetime.datetime(2034, 2, 26, 10, 30))
        self.assertEqual(imp.parse_sql_value("'2034-02-26T10:30:00'"),
                         datetime.datetime(2034, 2, 26, 10, 30))

    def test_string_normal(self):
        self.assertEqual(imp.parse_sql_value("'ABC123'"), "ABC123")

    def test_comillas_escapadas(self):
        self.assertEqual(imp.parse_sql_value("'texto''con''comillas'"),
                         "texto'con'comillas")

    def test_null_y_vacio(self):
        self.assertIsNone(imp.parse_sql_value("NULL"))
        self.assertIsNone(imp.parse_sql_value(""))

    def test_numeros_sin_comillas(self):
        self.assertEqual(imp.parse_sql_value("12345"), 12345)
        self.assertEqual(imp.parse_sql_value("2500000.00"), 2500000.0)

    def test_string_numerico_entre_comillas_no_es_numero(self):
        # Quoted: debe seguir siendo string (no se fuerza a int/float).
        self.assertEqual(imp.parse_sql_value("'12345'"), "12345")


class TestParseSqlInserts(unittest.TestCase):
    def test_no_doc_10_digitos_se_mantiene_como_clave_string(self):
        """REGRESIÓN end-to-end del parser: el INSERT de un cliente con no_doc
        numérico de 10 dígitos debe quedar como string en el rec (la clave de
        upsert), no como fecha."""
        dump = (
            "INSERT INTO clientes (TIPO_DOC, NO_DOC, NOMBRES, VENCIMIENTO_LICENCIA) "
            "VALUES ('CC', '1052070892', 'DEIVI JOSE', '2034-02-26');\n"
            "-- comentario con INSERT dentro que se ignora\n"
            "CREATE TABLE x (a INT);\n"
        )
        inserts = imp.parse_sql_inserts(dump)
        self.assertEqual(len(inserts), 1)
        tabla, rec, _ln = inserts[0]
        self.assertEqual(tabla, "CLIENTES")
        self.assertEqual(rec["NO_DOC"], "1052070892")
        self.assertIsInstance(rec["NO_DOC"], str)
        self.assertEqual(rec["VENCIMIENTO_LICENCIA"], datetime.date(2034, 2, 26))

    def test_inserts_de_autos_y_clientes_se_clasifican(self):
        dump = (
            "INSERT INTO autos (PLACA, MARCA) VALUES ('ABC123', 'Toyota');\n"
            "INSERT INTO clientes (NO_DOC, NOMBRES) VALUES ('1001234567', 'Juan');\n"
        )
        inserts = imp.parse_sql_inserts(dump)
        self.assertEqual([t for t, _, _ in inserts], ["AUTOS", "CLIENTES"])
        self.assertEqual(inserts[0][1]["PLACA"], "ABC123")
        self.assertEqual(inserts[1][1]["NO_DOC"], "1001234567")

    def test_otras_sentencias_se_ignoran(self):
        dump = "CREATE TABLE clientes (id INT);\nINSERT INTO otra (a) VALUES (1);\n"
        self.assertEqual(imp.parse_sql_inserts(dump), [])


if __name__ == "__main__":
    unittest.main(verbosity=2)
