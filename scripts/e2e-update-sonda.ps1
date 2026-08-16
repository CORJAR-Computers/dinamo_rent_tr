# e2e-update-sonda.ps1 - Sonda del updater: ejecuta updater_e2e (compilado con
# la version 1.0.12 del tag; misma pubkey y plugin que la app) contra el
# latest.json real de GitHub y guarda la salida en C:\shared\out\sonda-resultado.txt
# (visible en scripts/ del host).
#
# Para que sirve: separa "red/stack del Sandbox" de "la app instalada" cuando
# el check del auto-update no aparece. Si la sonda DETECTA la version nueva ->
# la red y el plugin funcionan (el problema esta en la app); si falla o no
# detecta -> el problema es de red dentro de la VM.
#
# NOTA: desde la v1.0.14 el check manual con el boton 'Buscar actualizacion'
# muestra el error real en pantalla (toast) - usar eso primero; la sonda queda
# como herramienta de segundo nivel (p. ej. una VM limpia sin app instalada).
#
# RECONSTRUIR LA SONDA (el binario no se commitea, es un build debug de 20 MB):
#   git worktree add /tmp/v1012 v1.0.12
#   cd /tmp/v1012/src-tauri && CARGO_TARGET_DIR=/tmp/v1012-target cargo build --bin updater_e2e
#   cp /tmp/v1012-target/debug/updater_e2e.exe C:\shared\bundle\updater_e2e_v1012.exe
#
# IMPORTANTE: C:\shared\bundle esta montado SOLO LECTURA en el .wsb. La sonda
# (binario debug) necesita VCRUNTIME140.dll y VCRUNTIME140_1.dll al lado, asi
# que se copian -junto con la sonda- a una carpeta temporal ESCRIBIBLE dentro
# de la VM (Join-Path $env:TEMP 'sonda-e2e') y se ejecuta desde ahi.
#
# Uso en el Sandbox:  powershell -ExecutionPolicy Bypass -File
# C:\shared\out\e2e-update-sonda.ps1

$ErrorActionPreference = 'Continue'
$sondaOrigen = 'C:\shared\bundle\updater_e2e_v1012.exe'
$out = 'C:\shared\out\sonda-resultado.txt'
$url = 'https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/latest/download/latest.json'
$dir = Join-Path $env:TEMP 'sonda-e2e'

$dlls = @('vcruntime140.dll', 'vcruntime140_1.dll', 'msvcp140.dll')
$origen = @(
    'C:\Users\WDAGUtilityAccount\AppData\Local\DinamoRent\firebird',
    'C:\Users\WDAGUtilityAccount\AppData\Local\DinamoRent',
    'C:\Program Files\DinamoRent\firebird',
    'C:\Program Files\DinamoRent'
)

$script:log = @()
function Write-Both($m) {
    Write-Host $m
    $script:log += $m
}

# Preparar carpeta temporal escribible (C:\shared\bundle es solo lectura)
if (Test-Path $dir) { Remove-Item -Recurse -Force $dir -ErrorAction SilentlyContinue }
New-Item -ItemType Directory -Force -Path $dir | Out-Null
$sonda = Join-Path $dir 'updater_e2e_v1012.exe'

if (-not (Test-Path $sondaOrigen)) {
    Write-Both "FALLO: no existe la sonda en $sondaOrigen"
    Set-Content -Path $out -Value ($script:log -join "`r`n") -Encoding UTF8
    exit 1
}
Copy-Item $sondaOrigen $sonda -Force

# Copiar las DLLs del runtime junto a la sonda (desde la carpeta de la app
# instalada, que el instalador si dejo).
$copiadas = 0
foreach ($d in $dlls) {
    foreach ($o in $origen) {
        $src = Join-Path $o $d
        if (Test-Path $src) {
            Copy-Item $src (Join-Path $dir $d) -Force -ErrorAction SilentlyContinue
            if (Test-Path (Join-Path $dir $d)) {
                $copiadas++
                Write-Both "OK: $d copiada junto a la sonda (desde $o)"
                break
            }
        }
    }
}
if ($copiadas -lt 2) {
    Write-Both "AVISO: solo $copiadas/3 DLLs del runtime - la sonda puede no arrancar"
}

Write-Both "=== Sonda updater_e2e v1.0.12 (dir temporal: $dir) ==="
Write-Both "Sonda presente: $([math]::Round((Get-Item $sonda).Length / 1MB, 1)) MB"
Write-Both "Ejecutando contra: $url"
Write-Both ""

$salida = & $sonda --endpoint $url 2>&1
$code = $LASTEXITCODE
$salida | ForEach-Object { Write-Both ($_ | Out-String).TrimEnd() }
Write-Both ""
Write-Both "EXIT CODE: $code"
Write-Both "=== fin ==="

Set-Content -Path $out -Value ($script:log -join "`r`n") -Encoding UTF8
Remove-Item -Recurse -Force $dir -ErrorAction SilentlyContinue
Write-Host ""
Write-Host "Resultado guardado en $out (host: scripts\sonda-resultado.txt)"
