#!/usr/bin/env bash
# verificar-updater-e2e.sh — Verifica de punta a punta el flujo de
# auto-actualización (tauri-plugin-updater) SIN publicar nada en GitHub:
#
#   1. Genera un artifact de prueba (instalador falso) y lo FIRMA con la clave
#      real (~/.tauri/dinamorent.key).
#   2. Arma un latest.json (v1.0.3 > actual 1.0.2) y lo sirve desde 127.0.0.1.
#   3. Ejecuta updater_e2e: check() debe detectar la v1.0.3 y download() debe
#      validar la firma contra la pubkey embebida en tauri.conf.json y devolver
#      los bytes exactos del artifact.
#   4. Caso negativo: latest.json con la misma versión (1.0.2) → sin actualización.
#
# Requiere: toolchain Rust, bun (con @tauri-apps/cli) y la clave privada de
# firma en ~/.tauri/dinamorent.key (la genera `bunx tauri signer generate`;
# NUNCA se commitea ni se sube al repo).
#
#   bash scripts/verificar-updater-e2e.sh
#
# Códigos de salida: 0 = todo OK · 1 = falló algo · 2 = uso incorrecto
set -u

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC_TAURI="$ROOT/src-tauri"
# cwd fijo en la raíz del repo: `bunx --no-install` busca node_modules/.bin/tauri
# subiendo por el árbol, así que el script funciona desde cualquier directorio.
cd "$ROOT"

# PATH con las rutas típicas de Windows (node/bun por npm -g, cargo por rustup).
NPM_GLOBAL="$(cygpath -u "${APPDATA:-}" 2>/dev/null)/npm"
[ -d "$NPM_GLOBAL" ] || NPM_GLOBAL="${APPDATA//\\\\//}/npm"
export PATH="/c/Program Files/nodejs:$HOME/.cargo/bin:$NPM_GLOBAL:$PATH"

KEY="$HOME/.tauri/dinamorent.key"
PUBKEY_FILE="$HOME/.tauri/dinamorent.key.pub"
VERSION_NUEVA="1.0.3"
VERSION_ACTUAL="1.0.2"
NOMBRE_ARTIFACT="DinamoRent_${VERSION_NUEVA}_x64-setup.exe"

echo "== Verificación del entorno =="

fallos=0
check() { # check <condición> <mensaje OK> <mensaje FALTA>
  if eval "$1"; then echo "  [OK]    $2"; else echo "  [FALTA] $3"; fallos=1; fi
}

check "command -v cargo >/dev/null 2>&1" "cargo $(cargo --version 2>/dev/null | cut -d' ' -f2)" "cargo — instala Rust: rustup-init"
check "command -v bun >/dev/null 2>&1" "bun $(bun --version 2>/dev/null)" "bun — instala bun"
check "[ -f \"$KEY\" ]" "clave privada en $KEY" "clave privada de firma — genera con: bunx tauri signer generate"
check "[ -f \"$PUBKEY_FILE\" ]" "pubkey en $PUBKEY_FILE" "archivo .pub de la clave — falta junto a la privada"

if [ "$fallos" -ne 0 ]; then
  echo "== Entorno incompleto, abortando =="
  exit 1
fi

echo "  Pubkey (firma del updater): $(head -c 40 "$PUBKEY_FILE" 2>/dev/null | tr -d '\n')…"

# ── Preparación de archivos de prueba (en un directorio temporal) ──────────────
TMP="$(mktemp -d /tmp/updater_e2e.XXXXXX 2>/dev/null || mktemp -d)"
SRV="$TMP/srv"
mkdir -p "$SRV/update" "$SRV/no-update"

INSTALLER="$SRV/update/$NOMBRE_ARTIFACT"
# Instalador falso: 1 MiB de bytes aleatorios (el contenido no importa, la
# integridad se prueba comparando bytes descargados vs servidos).
head -c 1048576 /dev/urandom > "$INSTALLER"
SHA256_ARTIFACT="$(sha256sum "$INSTALLER" | cut -d' ' -f1)"

SERVER_PID=""
cleanup() {
  if [ -n "$SERVER_PID" ] && kill -0 "$SERVER_PID" 2>/dev/null; then
    kill "$SERVER_PID" 2>/dev/null
    wait "$SERVER_PID" 2>/dev/null
  fi
  rm -rf "$TMP"
}
trap cleanup EXIT

echo "== 1/4 Firmando el artifact de prueba con la clave real =="
SIG_FILE="$INSTALLER.sig"
# -p "": la clave no tiene password, pero el signer igualmente pregunta por
# stdin (prompt de consola) y cuelga en entornos no interactivos (Git Bash).
if ! bunx --no-install tauri signer sign -f "$KEY" -p "" "$INSTALLER" >/dev/null 2>&1; then
  echo "❌ Falló la firma del artifact (bunx tauri signer sign)."
  exit 1
fi
if [ ! -s "$SIG_FILE" ]; then
  echo "❌ No se generó $SIG_FILE."
  exit 1
fi
echo "  Firmado: $SIG_FILE ($(wc -c < "$SIG_FILE") bytes)"

# Puerto libre para el servidor HTTP local.
PORT="$(python -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()' 2>/dev/null)"
[ -n "$PORT" ] || PORT=48321

echo "== 2/4 Armando latest.json (v$VERSION_NUEVA) y sirviéndolo en 127.0.0.1:$PORT =="
python - "$SIG_FILE" "$SRV/update/latest.json" "$SRV/no-update/latest.json" "$PORT" "$NOMBRE_ARTIFACT" <<'EOF'
import json, sys

sig_file, json_pos, json_neg, port, artifact = sys.argv[1:]
sig = open(sig_file, encoding="utf-8").read().strip()
url = f"http://127.0.0.1:{port}/update/{artifact}"

positivo = {
    "version": "1.0.3",
    "notes": "Release de prueba del E2E del updater (firma real, sin publicar en GitHub).",
    "pub_date": "2026-08-14T12:00:00Z",
    "platforms": {"windows-x86_64": {"signature": sig, "url": url}},
}
negativo = {
    "version": "1.0.2",
    "notes": "Misma version que la instalada: no debe ofrecer actualizacion.",
    "pub_date": "2026-08-14T12:00:00Z",
    "platforms": {"windows-x86_64": {"signature": sig, "url": url}},
}
with open(json_pos, "w", encoding="utf-8") as f:
    json.dump(positivo, f, indent=2)
with open(json_neg, "w", encoding="utf-8") as f:
    json.dump(negativo, f, indent=2)
print("  latest.json positivo -> v1.0.3")
print("  latest.json negativo -> v1.0.2")
EOF

python -m http.server "$PORT" --bind 127.0.0.1 --directory "$SRV" >/dev/null 2>&1 &
SERVER_PID=$!

# Esperar a que el servidor responda (máx ~10 s).
for _ in $(seq 1 50); do
  if curl -s -o /dev/null "http://127.0.0.1:$PORT/update/latest.json"; then break; fi
  sleep 0.2
done
if ! curl -s -o /dev/null "http://127.0.0.1:$PORT/update/latest.json"; then
  echo "❌ El servidor local no respondió en 127.0.0.1:$PORT."
  exit 1
fi

echo "== 3/4 Caso positivo: la app detecta la v$VERSION_NUEVA y valida la firma =="
if ! (cd "$SRC_TAURI" && cargo run -q --features dev --bin updater_e2e -- \
    --endpoint "http://127.0.0.1:$PORT/update/latest.json" \
    --expect-version "$VERSION_NUEVA" \
    --expect-file "$INSTALLER"); then
  echo "❌ Caso positivo FALLÓ."
  exit 1
fi

echo "== 4/4 Caso negativo: misma versión ($VERSION_ACTUAL) → sin actualización =="
if ! (cd "$SRC_TAURI" && cargo run -q --features dev --bin updater_e2e -- \
    --endpoint "http://127.0.0.1:$PORT/no-update/latest.json" \
    --expect-none); then
  echo "❌ Caso negativo FALLÓ."
  exit 1
fi

echo
echo "== Resultado: E2E del updater en verde =="
echo "  Artifact de prueba : $NOMBRE_ARTIFACT (sha256 $SHA256_ARTIFACT)"
echo "  Firma              : minisign (clave real de ~/.tauri/dinamorent.key)"
echo "  Detección v$VERSION_NUEVA : OK (check() + version_comparator)"
echo "  Verificación firma : OK (download() contra la pubkey de tauri.conf.json)"
echo "  Bytes descargados  : idénticos al artifact servido"
echo "  Sin actualización  : OK (v$VERSION_ACTUAL no ofrece update)"
echo
echo "Nota: esto valida el flujo local. Para el flujo real en GitHub hace falta"
echo "publicar la v$VERSION_NUEVA con el secret TAURI_SIGNING_PRIVATE_KEY"
echo "configurado (ver RELEASE_CHECKLIST.md) — tauri-action subirá latest.json"
echo "y los .sig, que es lo que la app instalada consulta al arrancar."
