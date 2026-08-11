#!/usr/bin/env bash
# backup-antes-rotacion.sh — Respaldos previos a la rotación de la clave PII
#
# Copia, para los despliegues de desarrollo y producción, el `config.ini`
# (que contiene `security.db_encryption_key`) y la BD Firebird `.fdb` en una
# carpeta con timestamp bajo `data/Backups/pre-rotacion/<YYYYmmdd-HHMMSS>/`,
# dejando un manifiesto con checksums sha256 para verificar la integridad.
#
# Sigue SECURITY.md §2.1 (Paso 0 — Backup). La app debe estar DETENIDA.
#
# Uso:
#   bash scripts/backup-antes-rotacion.sh              # dev + producción
#   bash scripts/backup-antes-rotacion.sh --dev-only   # solo desarrollo
#   bash scripts/backup-antes-rotacion.sh --prod-only  # solo producción (%APPDATA%)
#   bash scripts/backup-antes-rotacion.sh --dest DIR   # carpeta destino custom
#   bash scripts/backup-antes-rotacion.sh --force      # continuar con la app abierta
#
# ⚠️ El backup de `config.ini` contiene la clave PII: NO debe salir de esta
# máquina ni subirse a ningún repositorio (SECURITY.md §1.5 y §4).

set -euo pipefail

# ── Raíz del repo y despliegues ──────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DATA_DIR="$SCRIPT_DIR/data"

# La producción guarda config/BD en %APPDATA%/<identifier> (tauri.conf.json)
TAURI_IDENTIFIER="com.corjar.dinamorent"

DEV_ONLY=0
PROD_ONLY=0
FORCE=0
DEST=""

usage() {
  sed -n '2,19p' "$0" | sed 's/^# \{0,1\}//'
  exit 0
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dev-only) DEV_ONLY=1 ;;
    --prod-only) PROD_ONLY=1 ;;
    --force) FORCE=1 ;;
    --dest)
      DEST="${2:-}"
      if [[ -z "$DEST" ]]; then
        echo "Falta el valor de --dest" >&2
        exit 1
      fi
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

log() { printf '\033[1;36m[backup]\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m[aviso]\033[0m %s\n' "$*" >&2; }
die() { printf '\033[1;31m[error]\033[0m %s\n' "$*" >&2; exit 1; }

if [[ "$DEV_ONLY" -eq 1 && "$PROD_ONLY" -eq 1 ]]; then
  die "No se puede usar --dev-only y --prod-only a la vez."
fi

# ── Guarda: la copia de una BD abierta puede quedar inconsistente ────────────
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
  die "La app (dinamo-rent.exe) está corriendo. Ciérrala y reintenta, o usa --force si sabes lo que haces."
fi

# ── Ruta de la BD desde config.ini (database.path; relativa al data_dir) ─────
resolve_db() {
  local ini_dir="$1" ini_file="$2" path=""
  if [[ -f "$ini_file" ]]; then
    path="$(
      grep -E '^[[:space:]]*path[[:space:]]*=' "$ini_file" 2>/dev/null |
        head -1 |
        sed -E 's/^[^=]*=[[:space:]]*//' |
        tr -d '\r' |
        sed 's/[[:space:]]*$//' || true
    )"
  fi
  [[ -z "$path" ]] && path="dinamo_rent_v3.fdb"
  case "$path" in
    /* | [A-Za-z]:/* | [A-Za-z]:\\*) printf '%s' "$path" ;;
    *) printf '%s/%s' "$ini_dir" "$path" ;;
  esac
}

# ── Destino ──────────────────────────────────────────────────────────────────
if [[ -z "$DEST" ]]; then
  DEST="$DATA_DIR/Backups/pre-rotacion/$(date +%Y%m%d-%H%M%S)"
fi
mkdir -p "$DEST"

MANIFEST="$DEST/manifest.txt"
CHECKSUMS="$DEST/checksums.sha256"
: >"$MANIFEST"
: >"$CHECKSUMS"
{
  echo "Backup pre-rotación de la clave PII (SECURITY.md §2.1)"
  echo "Creado:      $(date '+%Y-%m-%d %H:%M:%S %Z')"
  echo "Destino:     $DEST"
  echo "Verificar:   sha256sum -c checksums.sha256 (dentro de esta carpeta)"
  echo ""
} >>"$MANIFEST"

# ── Copia un archivo, registra su checksum y restringe permisos ──────────────
backup_one() {
  local src="$1" name="$2"
  if [[ ! -f "$src" ]]; then
    warn "No existe (omitido): $src"
    return 0
  fi
  local out="$DEST/$name"
  cp -p "$src" "$out"
  # En POSIX se restringe a 0600 (config.ini contiene la clave). En Windows/Git-Bash
  # chmod es cosmético (core.filemode=false): los permisos reales los hereda la
  # carpeta destino de data/Backups (que no se commitea).
  if [[ "${OSTYPE:-}" != msys* && "${OSTYPE:-}" != cygwin* && "${OSTYPE:-}" != mingw* ]]; then
    chmod 600 "$out"
  fi
  (cd "$DEST" && sha256sum "$name") | tee -a "$CHECKSUMS"
  printf '%-30s %s\n' "$name" "$src" >>"$MANIFEST"
  log "OK  $src -> $DEST/$name"

  # Sanidad: el respaldo de config.ini debe contener la clave (no vacía)
  if [[ "$name" == config-*.ini ]]; then
    if grep -qE '^[[:space:]]*db_encryption_key[[:space:]]*=[[:space:]]*[^[:space:]]' "$out" 2>/dev/null; then
      log "OK  clave PII presente en $name"
    else
      warn "$name NO contiene db_encryption_key (o está vacía): este respaldo no sirve para una rotación."
    fi
  fi
}

# ── Ejecución ────────────────────────────────────────────────────────────────
if [[ "$PROD_ONLY" -eq 0 ]]; then
  log "== Desarrollo ($DATA_DIR) =="
  backup_one "$DATA_DIR/config.ini" "config-dev.ini"
  backup_one "$(resolve_db "$DATA_DIR" "$DATA_DIR/config.ini")" "dinamo-dev.fdb"
fi

if [[ "$DEV_ONLY" -eq 0 ]]; then
  if [[ -n "${APPDATA:-}" ]]; then
    PROD_DIR="$APPDATA/$TAURI_IDENTIFIER"
    if [[ -d "$PROD_DIR" ]]; then
      log "== Producción ($PROD_DIR) =="
      backup_one "$PROD_DIR/config.ini" "config-prod.ini"
      backup_one "$(resolve_db "$PROD_DIR" "$PROD_DIR/config.ini")" "dinamo-prod.fdb"
    else
      warn "No existe el directorio de producción $PROD_DIR (omitido)"
    fi
  else
    warn "APPDATA no definida (¿sistema no-Windows?): producción omitida"
  fi
fi

echo ""
log "Respaldo completado: $DEST"
log "Manifiesto:      $MANIFEST"
log "Checksums:       $CHECKSUMS  (verificar con: sha256sum -c \"$CHECKSUMS\")"
echo ""
echo "Restauración (si algo sale mal):"
echo "  cp \"$DEST/dinamo-dev.fdb\"  \"$DATA_DIR/\""
echo "  cp \"$DEST/config-dev.ini\"  \"$DATA_DIR/\""
echo ""
echo "  ⚠️  config-*.ini contiene la clave PII: no lo subas a ningún repositorio."
echo "  Guárdala también en tu gestor de contraseñas (SECURITY.md §1.5)."
