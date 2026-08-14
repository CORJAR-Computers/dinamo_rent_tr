#!/usr/bin/env bash
# test-completo.sh — Verifica el entorno de desarrollo y corre los tests del
# proyecto en un solo comando (Git Bash / MSYS2 en Windows).
#
#   bash scripts/test-completo.sh                 # lint + svelte-check + vitest + cargo test --lib
#   bash scripts/test-completo.sh --instalar      # + bun install primero
#   bash scripts/test-completo.sh --integra       # + tests de integración Rust (requiere BD dev)
#   bash scripts/test-completo.sh --solo-frontend # solo frontend
#   bash scripts/test-completo.sh --solo-backend  # solo backend
#
# Códigos de salida:
#   0 = todo OK · 1 = falló algo (env o tests) · 2 = uso incorrecto
set -u

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC_TAURI="$ROOT/src-tauri"

INSTALAR=0
INTEGRA=0
SOLO_FRONTEND=0
SOLO_BACKEND=0

for arg in "$@"; do
  case "$arg" in
    --instalar|-i) INSTALAR=1 ;;
    --integra)     INTEGRA=1 ;;
    --solo-frontend) SOLO_FRONTEND=1 ;;
    --solo-backend)  SOLO_BACKEND=1 ;;
    --ayuda|-h)    sed -n '2,10p' "$0"; exit 0 ;;
    *) echo "❌ Opción desconocida: $arg (usa --ayuda)"; exit 2 ;;
  esac
done

# Rutas típicas de Windows por si el shell actual no las tiene en PATH
# (node instalado por winget, bun por npm -g, cargo por rustup).
# Se convierten a formato MSYS (C:/... en vez de C:\...) para que bash las entienda.
NPM_GLOBAL="$(cygpath -u "${APPDATA:-}" 2>/dev/null)/npm"
[ -d "$NPM_GLOBAL" ] || NPM_GLOBAL="${APPDATA//\\//}/npm"
export PATH="/c/Program Files/nodejs:$HOME/.cargo/bin:$NPM_GLOBAL:$PATH"

fallos_env=0
ok_env()   { echo "  [OK]    $1"; }
falta_env() { echo "  [FALTA] $1 — $2"; fallos_env=1; }

echo "== Verificación del entorno =="

# node + npm
if command -v node >/dev/null 2>&1; then
  ok_env "node $(node --version)"
else
  falta_env "node" "instala Node LTS: winget install --id OpenJS.NodeJS.LTS -e"
fi
if command -v npm >/dev/null 2>&1; then
  ok_env "npm $(npm --version)"
else
  falta_env "npm" "viene con Node.js"
fi

# bun
if command -v bun >/dev/null 2>&1; then
  ok_env "bun $(bun --version)"
else
  falta_env "bun" "instala con: npm install -g bun"
fi

# cargo + rustc
if command -v cargo >/dev/null 2>&1; then
  ok_env "cargo $(cargo --version | awk '{print $2}')"
else
  falta_env "cargo" "instala con rustup: https://rustup.rs (toolchain stable-x86_64-pc-windows-msvc)"
fi
if command -v rustc >/dev/null 2>&1; then
  ok_env "rustc $(rustc --version | awk '{print $2}')"
else
  falta_env "rustc" "viene con rustup"
fi

# Linker MSVC (VS Build Tools con workload C++). Busca en todas las ediciones.
LINK_EXE=$(ls "/c/Program Files (x86)/Microsoft Visual Studio/2022/"*/VC/Tools/MSVC/*/bin/Hostx64/x64/link.exe 2>/dev/null | head -1)
if [ -n "$LINK_EXE" ]; then
  ok_env "link.exe (MSVC): ${LINK_EXE##*/VC/Tools/MSVC/}"
else
  falta_env "link.exe (MSVC)" "instala VS Build Tools 2022 con la workload C++: winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override \"--quiet --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended\""
fi

if [ "$fallos_env" -ne 0 ]; then
  echo ""
  echo "❌ Entorno incompleto — instala lo que falta y vuelve a correr el script."
  exit 1
fi
echo "  ✅ Entorno OK"

# ── Tests ──────────────────────────────────────────────────────────────────
cd "$ROOT" || exit 1
fallos_test=0
paso() { echo ""; echo "== $1 =="; }
fin_paso() { # $1 = exit code del paso, $2 = nombre
  if [ "$1" -eq 0 ]; then echo "  ✅ $2 OK"; else echo "  ❌ $2 FALLÓ"; fallos_test=1; fi
}

if [ "$INSTALAR" -eq 1 ]; then
  paso "bun install"
  bun install --frozen-lockfile
  fin_paso $? "bun install"
fi

if [ "$SOLO_BACKEND" -eq 0 ]; then
  paso "Lint (eslint)"
  bun run lint
  fin_paso $? "lint"

  paso "Svelte check"
  bunx svelte-kit sync
  bunx svelte-check --tsconfig ./tsconfig.json
  fin_paso $? "svelte-check"

  paso "Tests frontend (vitest)"
  bunx vitest run
  fin_paso $? "vitest"
fi

if [ "$SOLO_FRONTEND" -eq 0 ]; then
  paso "Tests backend (cargo test --lib)"
  (cd "$SRC_TAURI" && cargo test --lib)
  fin_paso $? "cargo test --lib"
fi

if [ "$INTEGRA" -eq 1 ] && [ "$SOLO_FRONTEND" -eq 0 ]; then
  if [ ! -f "$ROOT/data/dinamo_rent_v3.fdb" ]; then
    echo ""
    echo "⚠️  No existe la BD dev (data/dinamo_rent_v3.fdb). Créala primero con:"
    echo "    cd src-tauri && cargo run --features dev --bin sync_dev -- --solo-total"
    echo "    (los tests de integración 0016 y los de rentas con flota necesitan datos)"
  fi
  paso "Tests de integración (cargo test --tests)"
  (cd "$SRC_TAURI" && cargo test --tests)
  fin_paso $? "integración"
fi

echo ""
if [ "$fallos_test" -eq 0 ]; then
  echo "✅ TESTS COMPLETOS — todo verde"
  exit 0
else
  echo "❌ HUBO FALLOS (revisa la salida de cada paso)"
  exit 1
fi
