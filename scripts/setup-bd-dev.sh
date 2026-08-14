#!/usr/bin/env bash
# setup-bd-dev.sh — Deja la BD de desarrollo lista desde cero (Handsoff §6.3).
#
# Ejecuta los 7 pasos de la receta validada en un clon nuevo (sin
# data/config.ini ni data/dinamo_rent_v3.fdb). Todo es idempotente: se puede
# volver a correr sin romper nada.
#
#   1. sync_dev --solo-total   → crea config.ini + BD + aplica las 19 migraciones
#   2. paquetes Python         → firebird-driver, cryptography (para el importador)
#   3. clave PII               → genera db_encryption_key si está vacía
#   4. flota de prueba         → importar_autos_clientes.py con scripts/fixtures
#   5. admin                   → seed (verificar_instalacion_limpia) + reset a Admin123!
#   6. historial de auditoría  → LOGIN OK / LOGIN FALLIDO (lo exige auditoria_integration)
#   7. identidad de rentas     → próximo id >= 1000 (lo exige renta_no_contrato…)
#
#   bash scripts/setup-bd-dev.sh              # setup completo
#   bash scripts/setup-bd-dev.sh --verificar  # + cargo test --tests al final
#
# Códigos de salida: 0 = listo · 1 = falló algo · 2 = uso incorrecto
set -u

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC_TAURI="$ROOT/src-tauri"
DATA="$ROOT/data"
INI="$DATA/config.ini"
FDB="$DATA/dinamo_rent_v3.fdb"
FB_DIR="$SRC_TAURI/resources/firebird"
FIXTURES="$ROOT/scripts/fixtures/dump_autos_clientes.sql"

VERIFICAR=0
for arg in "$@"; do
  case "$arg" in
    --verificar) VERIFICAR=1 ;;
    --ayuda|-h) sed -n '2,20p' "$0"; exit 0 ;;
    *) echo "❌ Opción desconocida: $arg (usa --ayuda)"; exit 2 ;;
  esac
done

# PATH típico de Windows por si el shell actual no lo tiene (mismo patrón que
# test-completo.sh): node de winget, bun de npm -g, cargo de rustup.
NPM_GLOBAL="$(cygpath -u "${APPDATA:-}" 2>/dev/null)/npm"
[ -d "$NPM_GLOBAL" ] || NPM_GLOBAL="${APPDATA//\\//}/npm"
export PATH="/c/Program Files/nodejs:$HOME/.cargo/bin:$NPM_GLOBAL:$PATH"

# El driver de Firebird embedded necesita fbclient.dll en PATH. Se exporta en
# dos formatos: MSYS (para el PATH bash, lo hereda el importador) y Windows
# (para los heredocs python de los pasos 6/7).
export PATH="$FB_DIR:$PATH"
export DINAMO_FB_DIR="$(cygpath -w "$FB_DIR" 2>/dev/null || echo "$FB_DIR")"

paso() { echo ""; echo "== $1 =="; }

# Falla el script si un paso crítico falla (el setup no tiene sentido a medias).
requiere() { # $1 = exit code, $2 = nombre del paso
  if [ "$1" -ne 0 ]; then
    echo "  ❌ $2 FALLÓ — abortando (corrige y vuelve a correr; el script es idempotente)"
    exit 1
  fi
  echo "  ✅ $2 OK"
}

paso "1/7 Entorno: config.ini + BD + migraciones (sync_dev --solo-total)"
(cd "$SRC_TAURI" && cargo run --features dev --bin sync_dev -- --solo-total)
requiere $? "sync_dev"

paso "2/7 Paquetes Python (firebird-driver, cryptography)"
if python -c "import firebird.driver, cryptography" >/dev/null 2>&1; then
  echo "  ya instalados"
  requiere 0 "paquetes Python"
else
  python -m pip install --quiet firebird-driver cryptography
  requiere $? "pip install firebird-driver cryptography"
fi

paso "3/7 Clave PII (db_encryption_key en config.ini)"
python - "$INI" <<'EOF'
import configparser, sys
from cryptography.fernet import Fernet
ini = sys.argv[1]
cfg = configparser.ConfigParser()
cfg.read(ini)
if not cfg.has_section("security"):
    cfg.add_section("security")
actual = cfg.get("security", "db_encryption_key", fallback="").strip()
if actual:
    print("  ya configurada (se conserva)")
else:
    clave = Fernet.generate_key().decode()
    cfg.set("security", "db_encryption_key", clave)
    with open(ini, "w", encoding="utf-8") as f:
        cfg.write(f)
    print("  clave Fernet generada y escrita en [security] db_encryption_key")
EOF
requiere $? "clave PII"

paso "4/7 Flota de prueba (importar_autos_clientes.py con fixtures)"
(cd "$ROOT" && python scripts/importar_autos_clientes.py \
  --sql "$FIXTURES" --db "$FDB" --ini "$INI" --quiet)
requiere $? "dry-run importador"
(cd "$ROOT" && python scripts/importar_autos_clientes.py \
  --sql "$FIXTURES" --db "$FDB" --ini "$INI" --commit --quiet)
requiere $? "commit importador"

paso "5/7 Admin (seed si falta + reset a Admin123!)"
# El seed del admin solo ocurre en el arranque de la app o con el bin de
# verificación; verificar_instalacion_limpia además intenta login con la
# contraseña de fábrica, así que solo se corre cuando NO existe el admin
# (si ya se reseteó a Admin123!, su login fallaría y rompería la idempotencia).
python - "$INI" "$FDB" <<'EOF'
import configparser, os, sys
os.environ["PATH"] = os.environ.get("DINAMO_FB_DIR", "") + os.pathsep + os.environ.get("PATH", "")
from firebird.driver import connect, driver_config
driver_config.database_engine = "embedded"
ini, fdb = sys.argv[1], sys.argv[2]
cfg = configparser.ConfigParser()
cfg.read(ini)
user = cfg.get("database", "user", fallback="sysdba").strip()
pwd = cfg.get("database", "password", fallback="").strip()
con = connect(fdb, user=user, password=pwd, charset="UTF8")
cur = con.cursor()
n = cur.execute("SELECT COUNT(*) FROM usuarios WHERE username = 'admin'").fetchone()[0]
con.close()
print(f"  admin en BD: {'si' if n > 0 else 'no (hay que sembrarlo)'}")
sys.exit(0 if n > 0 else 3)
EOF
if [ $? -eq 0 ]; then
  echo "  (admin ya existe — se omite el seed)"
else
  (cd "$SRC_TAURI" && cargo run --features dev --bin verificar_instalacion_limpia -- "$DATA")
  requiere $? "seed admin"
fi
(cd "$SRC_TAURI" && cargo run --features dev --bin dev_reset_admin)
requiere $? "reset admin (Admin123!)"

paso "6/7 Historial de auditoría (LOGIN OK / LOGIN FALLIDO)"
python - "$INI" "$FDB" <<'EOF'
import configparser, os, sys
os.environ["PATH"] = os.environ.get("DINAMO_FB_DIR", "") + os.pathsep + os.environ.get("PATH", "")
from firebird.driver import connect, driver_config
driver_config.database_engine = "embedded"
ini, fdb = sys.argv[1], sys.argv[2]
cfg = configparser.ConfigParser()
cfg.read(ini)
user = cfg.get("database", "user", fallback="sysdba").strip()
pwd = cfg.get("database", "password", fallback="").strip()
con = connect(fdb, user=user, password=pwd, charset="UTF8")
cur = con.cursor()
for accion in ("LOGIN OK", "LOGIN FALLIDO"):
    n = cur.execute("SELECT COUNT(*) FROM auditoria WHERE accion = ?", (accion,)).fetchone()[0]
    if n == 0:
        cur.execute(
            "INSERT INTO auditoria (usuario, accion, mensaje, ip) VALUES (?, ?, ?, ?)",
            ("admin", accion, "seed setup-bd-dev", "127.0.0.1"),
        )
        print(f"  + evento {accion} insertado")
    else:
        print(f"  ya existe {accion} ({n})")
con.commit()
con.close()
EOF
requiere $? "historial de auditoría"

paso "7/7 Identidad de rentas (próximo id >= 1000)"
python - "$INI" "$FDB" <<'EOF'
import configparser, os, sys
os.environ["PATH"] = os.environ.get("DINAMO_FB_DIR", "") + os.pathsep + os.environ.get("PATH", "")
from firebird.driver import connect, driver_config
driver_config.database_engine = "embedded"
ini, fdb = sys.argv[1], sys.argv[2]
cfg = configparser.ConfigParser()
cfg.read(ini)
user = cfg.get("database", "user", fallback="sysdba").strip()
pwd = cfg.get("database", "password", fallback="").strip()
con = connect(fdb, user=user, password=pwd, charset="UTF8")
cur = con.cursor()
max_id = cur.execute("SELECT COALESCE(MAX(id), 0) FROM rentas").fetchone()[0]
siguiente = max(1000, max_id + 1)
cur.execute(f"ALTER TABLE rentas ALTER COLUMN id RESTART WITH {siguiente}")
con.commit()
print(f"  identidad de rentas: próximo id {siguiente}")
con.close()
EOF
requiere $? "identidad de rentas"

if [ "$VERIFICAR" -eq 1 ]; then
  paso "Verificación: cargo test --tests"
  (cd "$SRC_TAURI" && cargo test --tests)
  requiere $? "cargo test --tests"
fi

echo ""
echo "✅ BD dev lista — cargo test --tests debería correr completo en verde"
echo "   (si un test de rentas/comparendos/mantenimiento/gastos falla por falta"
echo "   de flota, el mensaje ya indica que hay que correr este script)."
exit 0
