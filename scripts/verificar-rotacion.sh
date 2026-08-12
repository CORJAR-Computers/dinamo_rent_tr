#!/usr/bin/env bash
# verificar-rotacion.sh — Política de verificación post-rotación de la clave PII
#
# Tras una rotación de clave (§2.1), este script ejecuta las verificaciones que
# SECURITY.md §2.1 Paso 4 exige para dar la rotación por VÁLIDA:
#
#   1. [ANTI DOBLE-CIFRADO, Paso 4.5] Dry-run de `normalizar_doble_cifrado.py`
#      sobre la(s) BD(s): debe reportar **0 campos con cifrado anidado** y
#      **0 indescifrables**. Si >0, la rotación re-duplicó capas (regresión del
#      binario) y el script falla (exit != 0), indicando recuperar con §5.3.
#
#   2. [AUDITORÍA, Paso 4.4] (opcional, con --exige-evento-rotacion) La tabla
#      `auditoria` debe contener al menos un evento `PII_KEY_ROTATED` (usuario
#      `sistema`, ip `local`) registrado por el bin en la misma transacción.
#
# Es una POLÍTICA automatizable: devuelve exit 0 solo si TODO es válido, así
# puede usarse como gate en CI o en el checklist manual post-rotación.
#
# Uso:
#   bash scripts/verificar-rotacion.sh                    # dev + producción
#   bash scripts/verificar-rotacion.sh --dev-only         # solo desarrollo
#   bash scripts/verificar-rotacion.sh --prod-only        # solo producción (%APPDATA%)
#   bash scripts/verificar-rotacion.sh --db RUTA --ini RUTA   # BD/config custom
#   bash scripts/verificar-rotacion.sh --exige-evento-rotacion # + auditoría PII_KEY_ROTATED
#   bash scripts/verificar-rotacion.sh --force            # continuar con la app abierta
#
# ⚠️ Firebird embedded permite UNA conexión por BD: la app debe estar DETENIDA
# (igual que para el backup §2.1 Paso 0), salvo que se use --force.

set -euo pipefail

# ── Raíz del repo y despliegues ──────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NORMALIZAR="$SCRIPT_DIR/scripts/normalizar_doble_cifrado.py"
FIREBIRD_DIR="$SCRIPT_DIR/src-tauri/resources/firebird"
TAURI_IDENTIFIER="com.corjar.dinamorent"

DEV_ONLY=0
PROD_ONLY=0
FORCE=0
EXIGE_EVENTO=0
DB_CUSTOM=""
INI_CUSTOM=""

usage() {
  sed -n '2,24p' "$0" | sed 's/^# \{0,1\}//'
  exit 0
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dev-only) DEV_ONLY=1 ;;
    --prod-only) PROD_ONLY=1 ;;
    --force) FORCE=1 ;;
    --exige-evento-rotacion) EXIGE_EVENTO=1 ;;
    --db)
      DB_CUSTOM="${2:-}"
      [[ -n "$DB_CUSTOM" ]] || { echo "Falta el valor de --db" >&2; exit 1; }
      shift
      ;;
    --ini)
      INI_CUSTOM="${2:-}"
      [[ -n "$INI_CUSTOM" ]] || { echo "Falta el valor de --ini" >&2; exit 1; }
      shift
      ;;
    -h | --help) usage ;;
    *)
      echo "Opción desconocida: $1 (usa --help)" >&2
      exit 1
      ;;
  esac
  shift
done

if [[ "$DEV_ONLY" -eq 1 && "$PROD_ONLY" -eq 1 ]]; then
  echo "[error] No se puede usar --dev-only y --prod-only a la vez." >&2
  exit 1
fi

log() { printf '\033[1;36m[verificar]\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m[aviso]\033[0m %s\n' "$*" >&2; }
die() { printf '\033[1;31m[error]\033[0m %s\n' "$*" >&2; exit 1; }

# ── Guarda: la verificación necesita acceso exclusivo a la BD embedded ───────
app_running() {
  if command -v tasklist >/dev/null 2>&1; then
    tasklist 2>/dev/null | grep -qi 'dinamo-rent' && return 0
  fi
  if command -v pgrep >/dev/null 2>&1; then
    pgrep -f 'dinamo-rent' >/dev/null 2>&1 && return 0
  fi
  return 1
}

if app_running && [[ "$FORCE" -ne 1 ]]; then
  die "La app (dinamo-rent.exe) está corriendo y bloquea la BD embedded. Ciérrala y reintenta, o usa --force si sabes lo que haces."
fi

# ── Utilidades ───────────────────────────────────────────────────────────────
# Ejecuta el dry-run contra una BD y valida que no quede cifrado anidado.
verificar_dry_run() {
  local db="$1" ini="$2" etiqueta="$3"
  log "== $etiqueta: $db =="

  if [[ ! -f "$db" ]]; then
    warn "No existe la BD (omitida): $db"
    return 0
  fi

  local salida
  salida="$(python "$NORMALIZAR" --db "$db" --ini "$ini" 2>&1)" || {
    echo "$salida" | grep -E 'ERROR|Error|error' | head -5 >&2
    die "El dry-run de normalización falló sobre $etiqueta."
  }

  echo "$salida" | grep -aE 'anidado a normalizar|indescifrables|DRY-RUN|Reporte' | head -6

  local anidados indescifrables
  anidados="$(echo "$salida" | grep -aoE 'anidado a normalizar: [0-9]+' | grep -aoE '[0-9]+$' || echo '?')"
  indescifrables="$(echo "$salida" | grep -aoE 'indescifrables \(se dejaron\): *[0-9]+' | grep -aoE '[0-9]+$' || echo '?')"

  if [[ "$anidados" == "?" || "$indescifrables" == "?" ]]; then
    die "No se pudo parsear el reporte del dry-run sobre $etiqueta (posible bug del script o salida inesperada)."
  fi

  if [[ "$anidados" != "0" ]]; then
    die "POLÍTICA §2.1 Paso 4.5 VIOLADA en $etiqueta: quedan $anidados campos con cifrado anidado. La rotación re-duplicó capas — recuperar con §5.3."
  fi
  if [[ "$indescifrables" != "0" ]]; then
    die "POLÍTICA §2.1 Paso 4.5 VIOLADA en $etiqueta: $indescifrables campos indescifrables con la clave actual."
  fi
  log "OK  $etiqueta: 0 campos anidados, 0 indescifrables."
}

# Verifica que exista al menos un evento PII_KEY_ROTATED en auditoria.
verificar_evento_rotacion() {
  local db="$1" ini="$2" etiqueta="$3"
  log "== Auditoría ($etiqueta): buscando PII_KEY_ROTATED =="
  local total
  total="$(FIREBIRD_DIR="$FIREBIRD_DIR" python - "$db" "$ini" <<'PYEOF'
import configparser, os, sys
os.environ["PATH"] = os.environ["FIREBIRD_DIR"] + os.pathsep + os.environ.get("PATH", "")
from firebird.driver import connect, driver_config
config = driver_config
config.database_engine = "embedded"
db, ini = sys.argv[1], sys.argv[2]
cfg = configparser.ConfigParser()
cfg.read(ini)
con = connect(db, user=cfg.get("database", "user", fallback="sysdba"), password=cfg.get("database", "password", fallback=""), charset="UTF8")
cur = con.cursor()
cur.execute("SELECT COUNT(*) FROM auditoria WHERE accion = 'PII_KEY_ROTATED'")
print(cur.fetchone()[0])
con.close()
PYEOF
)" || die "No se pudo consultar auditoria de $etiqueta."

  if [[ "$total" == "0" ]]; then
    die "POLÍTICA §2.1 Paso 4.4 VIOLADA en $etiqueta: no existe ningún evento PII_KEY_ROTATED en la tabla auditoria."
  fi
  log "OK  $etiqueta: $total evento(s) PII_KEY_ROTATED registrado(s)."
}

# ── Determinación de objetivos ───────────────────────────────────────────────
# Las funciones abortan con die() ante cualquier fallo (exit != 0); si se llega
# al final sin error, la política está CUMPLIDA.
if [[ -n "$DB_CUSTOM" && -n "$INI_CUSTOM" ]]; then
  verificar_dry_run "$DB_CUSTOM" "$INI_CUSTOM" "custom"
  if [[ "$EXIGE_EVENTO" -eq 1 ]]; then
    verificar_evento_rotacion "$DB_CUSTOM" "$INI_CUSTOM" "custom"
  fi
else
  if [[ "$PROD_ONLY" -eq 0 ]]; then
    verificar_dry_run "$SCRIPT_DIR/data/dinamo_rent_v3.fdb" "$SCRIPT_DIR/data/config.ini" "dev"
    if [[ "$EXIGE_EVENTO" -eq 1 ]]; then
      verificar_evento_rotacion "$SCRIPT_DIR/data/dinamo_rent_v3.fdb" "$SCRIPT_DIR/data/config.ini" "dev"
    fi
  fi
  if [[ "$DEV_ONLY" -eq 0 && -n "${APPDATA:-}" ]]; then
    local_prod_dir="$APPDATA/$TAURI_IDENTIFIER"
    if [[ -d "$local_prod_dir" ]]; then
      verificar_dry_run "$local_prod_dir/dinamo_rent_v3.fdb" "$local_prod_dir/config.ini" "prod"
      if [[ "$EXIGE_EVENTO" -eq 1 ]]; then
        verificar_evento_rotacion "$local_prod_dir/dinamo_rent_v3.fdb" "$local_prod_dir/config.ini" "prod"
      fi
    else
      warn "No existe el directorio de producción $local_prod_dir (omitido)"
    fi
  fi
fi

echo ""
log "POLÍTICA §2.1 Paso 4 CUMPLIDA: sin cifrado anidado, sin indescifrables."
if [[ "$EXIGE_EVENTO" -eq 1 ]]; then
  log "  + Auditoría PII_KEY_ROTATED presente."
fi
exit 0
