#!/usr/bin/env python3
"""generar_excel_ejemplo.py — Genera scripts/fixtures/datos_autos_clientes.xlsx
(hojas "autos" y "clientes") para probar scripts/importar_autos_clientes.py.

Uso: python scripts/fixtures/generar_excel_ejemplo.py
"""
from openpyxl import Workbook
from openpyxl.styles import Font

wb = Workbook()

# Hoja autos
ws = wb.active
ws.title = "autos"
ws.append(["Placa", "Marca", "Modelo", "Color", "Tipo", "Estado",
           "Costo Fijo Mensual", "Kilometraje", "Vencimiento SOAT", "Fecha Ingreso"])
ws.append(["DEF456", "Renault", "Duster", "Gris", "SUV", "Disponible",
           2800000.0, 32000, "2026-07-01", "2024-05-20"])
ws.append(["GHI012", "Mazda", "CX-5", "Negro", "SUV", "Mantenimiento",
           3100000.0, 51000, "2026-02-28", "2024-02-10"])

# Hoja clientes
ws2 = wb.create_sheet("clientes")
ws2.append(["Tipo Doc", "No Doc", "Nombres", "Apellidos", "Celular", "Email",
            "Ciudad", "País", "Dirección", "No Licencia", "Vencimiento Licencia"])
ws2.append(["CC", "1005556677", "Carlos", "López Díaz", "3122223344",
            "carlos.lopez@example.com", "Cali", "Colombia", "Calle 5 # 20-10",
            "LC5556667778", "2027-03-01"])
ws2.append(["CC", "1008889900", "Ana", "Martínez Ruiz", "3153334455",
            "ana.martinez@example.com", "Barranquilla", "Colombia", "Cra 30 # 8-90",
            "LC8889990001", "2026-08-15"])

# Estilo básico de encabezados
for ws_ in (ws, ws2):
    for cell in ws_[1]:
        cell.font = Font(bold=True)

wb.save("scripts/fixtures/datos_autos_clientes.xlsx")
print("OK: scripts/fixtures/datos_autos_clientes.xlsx generado")
