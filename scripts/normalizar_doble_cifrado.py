#!/usr/bin/env python3
"""normalizar_doble_cifrado.py — Des-hace el doble cifrado PII de la BD dev.

Contexto: la rotación de clave del 2026-08-10 se ejecutó con un binario con bug
que re-cifraba tokens `v1:` ya cifrados (doble capa con la MISMA clave). La app
descifra una sola capa y muestra la capa interna como texto (tokens visibles en
el modal de edición).

Este script:
  1. Lee cada columna PII de clientes.
  2. Des-envuelve TODAS las capas v1:/Fernet con la clave actual hasta el texto
     en claro (con límite de seguridad).
  3. Re-cifra UNA sola vez con la clave actual (formato sano = BD de producción).
  4. Escribe en una transacción y deja un reporte de verificación.

Sigue SECURITY.md §2.1: ejecutar SOLO con respaldo previo y app DETENIDA.

Uso:
  python scripts/normalizar_doble_cifrado.py [--db RUTA] [--commit]
Por defecto solo analiza (dry-run). Con --commit escribe los cambios.
"""
import argparse
import base64
import hashlib
import os
import sys

os.environ["PATH"] = (
    r"D:\dinamo_rent_tr\src-tauri\resources\firebird"
    + os.pathsep
    + os.environ.get("PATH", "")
)

from firebird.driver import connect, driver_config  # noqa: E402
from cryptography.hazmat.primitives.ciphers.aead import AESGCM  # noqa: E402

config = driver_config
config.database_engine = "embedded"

COLS_PII = [
    "CELULAR",
    "CELULAR2",
    "EMAIL",
    "DIR_RESIDENCIA",
    "DIR_TEMPORAL",
    "NO_LICENCIA",
]
MAX_CAPAS = 10


def load_key(ini_path):
    import configparser

    cfg = configparser.ConfigParser()
    cfg.read(ini_path)
    user = cfg.get("database", "user", fallback="sysdba").strip()
    pwd = cfg.get("database", "password", fallback="").strip()
    key = cfg.get("security", "db_encryption_key", fallback="").strip()
    return user, pwd, key


def decrypt_v1(aes_key: bytes, tok: str) -> str:
    body = tok[3:]
    nonce_b64, ct_b64 = body.split(":", 1)
    nonce = base64.b64decode(nonce_b64)
    ct = base64.b64decode(ct_b64)
    plain = AESGCM(aes_key).decrypt(nonce, ct, None)
    return plain.decode("utf-8")


def is_v1(v: str) -> bool:
    return v.startswith("v1:")


def unwrap(aes_key: bytes, valor, reporte):
    """Devuelve (texto_en_claro, capas_quitadas, ok, mensaje)."""
    if valor is None or valor == "":
        return valor, 0, True, "vacio"
    if not isinstance(valor, str):
        return valor, 0, True, "no-str"
    capas = 0
    actual = valor
    while is_v1(actual) and capas < MAX_CAPAS:
        try:
            actual = decrypt_v1(aes_key, actual)
            capas += 1
        except Exception as e:  # noqa: BLE001
            reporte["indescifrables"] += 1
            return actual, capas, False, f"v1-indescifrable:{type(e).__name__}"
    # Si el bucle terminó por el límite y sigue siendo token → alerta
    if is_v1(actual) and capas >= MAX_CAPAS:
        reporte["indescifrables"] += 1
        return actual, capas, False, "limite-capas"
    return actual, capas, True, "ok"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--db", default=r"D:\dinamo_rent_tr\data\dinamo_rent_v3.fdb")
    parser.add_argument("--ini", default=r"D:\dinamo_rent_tr\data\config.ini")
    parser.add_argument("--commit", action="store_true", help="Escribe los cambios")
    args = parser.parse_args()

    user, pwd, key_str = load_key(args.ini)
    if not key_str:
        print("ERROR: db_encryption_key vacía en config.ini — abortando.")
        sys.exit(2)
    aes_key = hashlib.sha256(key_str.encode()).digest()

    con = connect(args.db, user=user, password=pwd, charset="UTF8")
    cur = con.cursor()
    cur.execute(f"SELECT ID, {', '.join(COLS_PII)} FROM clientes")
    rows = cur.fetchall()

    reporte = {"indescifrables": 0, "cambiados": 0, "capas_totales": 0}
    planes = []  # (id, col, valor_original, valor_nuevo, capas)

    for r in rows:
        cid = r[0]
        for col, valor in zip(COLS_PII, r[1:]):
            if not isinstance(valor, str) or not is_v1(valor):
                continue
            claro, capas, ok, msg = unwrap(aes_key, valor, reporte)
            if not ok:
                print(f"  ! cliente {cid} {col}: {msg} (se deja intacto)")
                continue
            if capas == 0:
                continue
            # Solo normalizar si hay MÁS de una capa (cifrado doble/anidado).
            # Un token v1: sano (1 capa) se deja tal cual.
            if capas == 1:
                continue
            # Re-cifrar UNA sola vez
            nuevo = encrypt(aes_key, claro)
            reporte["cambiados"] += 1
            reporte["capas_totales"] += capas
            planes.append((cid, col, valor, nuevo, capas))

    if args.commit:
        print(f"== Escribiendo {len(planes)} campos en transacción...")
        try:
            c2 = con.cursor()
            for cid, col, _viejo, nuevo, capas in planes:
                c2.execute(
                    f"UPDATE clientes SET {col} = ? WHERE ID = ?",
                    (nuevo, cid),
                )
            # Auditoría PII_NORMALIZADA en el mismo commit (atómico): usuario
            # sistema, ip local, mensaje con conteos SIN exponer la clave ni
            # datos PII. Solo si hubo cambios (evita ruido en re-ejecuciones).
            if planes:
                c2.execute(
                    "INSERT INTO auditoria (usuario, accion, mensaje, ip, fecha) "
                    "VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP)",
                    (
                        "sistema",
                        "PII_NORMALIZADA",
                        f"Normalización de PII completada: {len(planes)} campos "
                        f"re-cifrados a una sola capa ({reporte['capas_totales']} capas "
                        "eliminadas), 0 claves expuestas",
                        "local",
                    ),
                )
            con.commit()
            print("== Commit OK (incluye auditoría PII_NORMALIZADA si hubo cambios) ==")
        except Exception as e:  # noqa: BLE001
            con.rollback()
            print(f"== ERROR: se revirtió la transacción — {type(e).__name__}: {e}")
            con.close()
            sys.exit(1)
    else:
        print("== DRY-RUN: no se escribió nada ==")

    print("\n=== Reporte ===")
    print(f"  campos PII con cifrado anidado a normalizar: {len(planes)}")
    print(f"  campos re-cifrados a una sola capa:          {reporte['cambiados']}")
    print(f"  capas totales eliminadas:                    {reporte['capas_totales']}")
    print(f"  campos indescifrables (se dejaron):          {reporte['indescifrables']}")
    if not args.commit:
        print("\n  Re-ejecuta con --commit para aplicar los cambios.")

    con.close()


def encrypt(aes_key: bytes, plaintext: str) -> str:
    if not plaintext:
        return plaintext
    import secrets

    nonce = secrets.token_bytes(12)
    cipher = AESGCM(aes_key)
    ct = cipher.encrypt(nonce, plaintext.encode("utf-8"), None)
    return f"v1:{base64.b64encode(nonce).decode()}:{base64.b64encode(ct).decode()}"


if __name__ == "__main__":
    main()
