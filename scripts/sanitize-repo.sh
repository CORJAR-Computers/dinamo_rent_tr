#!/usr/bin/env bash
# ============================================================================
# sanitize-repo.sh — Saneamiento del working tree e índice Git del repo
#                    dinamo_rent_tr (Grupo A — Seguridad y saneamiento).
#
# QUÉ HACE:
#   1. Borra del disco la carpeta Firebird-5.0.3.1683-0-windows-x64/ (copia
#      duplicada — el bundle real está en src-tauri/resources/firebird/).
#   2. git rm --cached de archivos que NO deben estar en el índice pero que
#      sí pueden existir en el working tree del desarrollador:
#         data/dinamo_rent_v3.fdb        (BD Firebird, binaria)
#         data/config.ini                (contiene secretos)
#         Contrato_Dinamo.docx           (artefacto de negocio)
#         informe_*.xlsx                 (artefactos de negocio)
#         static/preview-shots/*.pdf     (previews de impresión)
#      Importante: --cached NO borra del disco, solo del índice. El working
#      tree conserva los archivos locales.
#   3. Imprime instrucciones para purgar el historial Git con git filter-repo
#      (necesario si secretos estuvieron commiteados en el pasado).
#
# NO HACE:
#   - No rota la clave PII (ver SECURITY.md §2).
#   - No purga el historial automáticamente (solo imprime instrucciones).
#   - No modifica archivos de código.
#
# USO:
#   bash scripts/sanitize-repo.sh           # modo dry-run (solo imprime)
#   bash scripts/sanitize-repo.sh --yes     # ejecuta realmente
#
# SALIDA:
#   - Código 0 si todo OK (o si dry-run se completó).
#   - Código 1 si no se pasó --yes en modo real, o si ocurre un error.
#
# REQUIERE:
#   - Bash 4+ (testado en 5.x).
#   - Git en PATH.
#   - Estar en la raíz del repo (o pasar --repo /ruta/al/repo).
# ============================================================================

set -euo pipefail

# ---------- Helpers ----------
log()    { printf '[sanitize] %s\n' "$*"; }
warn()   { printf '[sanitize] ⚠️  %s\n' "$*" >&2; }
err()    { printf '[sanitize] ❌ %s\n' "$*" >&2; }
ok()     { printf '[sanitize] ✅ %s\n' "$*"; }

# ---------- Args ----------
CONFIRMED=0
REPO_PATH="."

while [[ $# -gt 0 ]]; do
  case "$1" in
    --yes|-y)
      CONFIRMED=1
      shift
      ;;
    --repo)
      REPO_PATH="$2"
      shift 2
      ;;
    --help|-h)
      sed -n '2,40p' "$0"
      exit 0
      ;;
    *)
      err "Argumento desconocido: $1"
      exit 1
      ;;
  esac
done

cd "$REPO_PATH"

# Verificar que estamos en un repo git
if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  err "No estás dentro de un repositorio Git (cwd=$(pwd))."
  err "Usa: bash scripts/sanitize-repo.sh --repo /ruta/al/repo"
  exit 1
fi

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"
log "Repo root: $REPO_ROOT"

if [[ "$CONFIRMED" -eq 0 ]]; then
  log "MODO DRY-RUN (no se hacen cambios). Pasar --yes para ejecutar."
  log "Las acciones que SE EJECUTARÍAN con --yes son:"
else
  log "MODO REAL — se aplicarán cambios."
fi

echo
log "---- Acción 1: borrar Firebird-5.0.3.1683-0-windows-x64/ ----"
FB_DIR="Firebird-5.0.3.1683-0-windows-x64"
if [[ -d "$FB_DIR" ]]; then
  SIZE=$(du -sh "$FB_DIR" 2>/dev/null | awk '{print $1}')
  if [[ "$CONFIRMED" -eq 1 ]]; then
    rm -rf "$FB_DIR"
    ok "Borrado $FB_DIR ($SIZE liberados)."
  else
    log "  Se borraría: $FB_DIR ($SIZE)"
  fi
else
  log "  (no existe $FB_DIR — omitido)"
fi

echo
log "---- Acción 2: git rm --cached de artefactos no commiteables ----"

# Lista de patrones a sacar del índice (no se borran del disco)
PATTERNS=(
  "data/dinamo_rent_v3.fdb"
  "data/config.ini"
  "Contrato_Dinamo.docx"
  "informe_*.xlsx"
  "static/preview-shots/*.pdf"
)

for pat in "${PATTERNS[@]}"; do
  # Listar qué archivos del índice coincen con el patrón
  # shellcheck disable=SC2207
  matches=($(git ls-files --cached -- "$pat" 2>/dev/null || true))
  if [[ ${#matches[@]} -eq 0 ]]; then
    log "  ($pat) — no está en el índice, omitido"
    continue
  fi
  for m in "${matches[@]}"; do
    if [[ "$CONFIRMED" -eq 1 ]]; then
      git rm --cached --quiet -- "$m"
      ok "git rm --cached $m (sigue en disco: $([ -f "$m" ] && echo sí || echo no))"
    else
      log "  Se haría: git rm --cached -- $m"
    fi
  done
done

echo
if [[ "$CONFIRMED" -eq 1 ]]; then
  ok "Acciones completadas. Revisa con: git status"
  log "Recuerda commitear el cambio de índice: git commit -m 'chore: untrack artifacts and secrets from index'"
else
  log "Dry-run finalizado. Ejecuta de nuevo con --yes para aplicar."
fi

echo
log "=========================================================================="
log "  PURGA DEL HISTORIAL GIT (manual — no automatizada)"
log "=========================================================================="
log ""
log "Si en algún momento estos archivos estuvieron commiteados (especialmente"
log "data/config.ini con secretos), el historial Git sigue conteniéndolos y se"
log "debe purgar con git-filter-repo:"
log ""
log "  # 1. Instalar git-filter-repo (one-time)"
log "  pip install --user git-filter-repo"
log "  #   o: apt install git-filter-repo  /  brew install git-filter-repo"
log ""
log "  # 2. Hacer un backup del repo ANTES (git-filter-repo reescribe historia)"
log "  cp -a ../dinamo_rent_tr ../dinamo_rent_tr.backup-pre-purge"
log ""
log "  # 3a. Eliminar archivos sensibles del historial completo:"
log "  git filter-repo --invert-paths \\"
log "    --path data/config.ini \\"
log "    --path data/dinamo_rent_v3.fdb \\"
log "    --path Contrato_Dinamo.docx \\"
log "    --path-glob 'informe_*.xlsx' \\"
log "    --path-glob 'static/preview-shots/*.pdf'"
log ""
log "  # 3b. Alternativa: reemplazar SOLO el valor de la clave expuesta:"
log "  echo 'REDACTED_OLD_KEY==>REDACTED_OLD_KEY' > /tmp/replacements.txt"
log "  git filter-repo --replace-text /tmp/replacements.txt"
log ""
log "  # 4. Force-push a TODOS los remotos (¡coordina con el equipo primero!)"
log "  git remote -v"
log "  git push --force --all"
log "  git push --force --tags"
log ""
log "  # 5. Pedir a cada desarrollador que RE-CLONE el repo (los clones"
log "  #    existentes siguen teniendo el historial viejo con el secreto)."
log ""
log "  # 6. Rota la clave PII (SECURITY.md §2) y la contraseña sysdba de"
log "  #    Firebird si se migró a server. Aunque purgues el historial,"
log "  #    asume que la clave vieja está comprometida."
log ""
log "  Referencia: SECURITY.md §2 (rotación) y §4 (incidente histórico)."
log "=========================================================================="

if [[ "$CONFIRMED" -eq 0 ]]; then
  exit 0
fi

ok "Saneamiento completado."
exit 0
