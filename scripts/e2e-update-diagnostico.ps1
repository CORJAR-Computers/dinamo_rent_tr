# e2e-update-diagnostico.ps1 - Diagnostico rapido para la prueba E2E del
# auto-update dentro del Sandbox. ASCII puro a proposito.
#
# Verifica:
#   1. Conectividad al latest.json real de GitHub (el updater lo consulta).
#   2. Que la app v1.0.12 instalada arranque y siga viva.
#   3. Las ventanas visibles (si el dialogo 'Actualizacion disponible' con
#      tilde esta en pantalla, aparece aqui).
#
# Uso en el Sandbox:  powershell -ExecutionPolicy Bypass -File
# C:\shared\out\e2e-update-diagnostico.ps1

$ErrorActionPreference = 'Continue'
$exe = "$env:LOCALAPPDATA\DinamoRent\dinamo-rent.exe"
$url = 'https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/latest/download/latest.json'

function Write-Diag([string]$msg) {
    Write-Host ("[{0}] {1}" -f (Get-Date -Format 'HH:mm:ss'), $msg)
}

Write-Diag "=== Diagnostico E2E auto-update ==="

# 1) Conectividad
Write-Diag "1) Probando conexion al latest.json de GitHub..."
try {
    $r = Invoke-WebRequest -Uri $url -UseBasicParsing -TimeoutSec 20
    Write-Diag "   OK: HTTP $($r.StatusCode), $($r.Content.Length) bytes"
    $m = $r.Content | ConvertFrom-Json
    Write-Diag "   Version declarada en latest.json: $($m.version)"
} catch {
    Write-Diag "   FALLO conectividad: $($_.Exception.Message)"
}

# 2) Version instalada
if (Test-Path $exe) {
    $v = (Get-Item $exe).VersionInfo.ProductVersion
    Write-Diag "2) App instalada: $exe (producto $v)"
} else {
    Write-Diag "2) FALLO: no existe $exe"
    exit 1
}

# 3) Arrancar la app y listar ventanas
Write-Diag "3) Arrancando la app y esperando 15 s (check del updater)..."
$p = Start-Process -FilePath $exe -PassThru
Start-Sleep -Seconds 15
$p.Refresh()
if ($p.HasExited) {
    Write-Diag "   FALLO: la app salio sola con codigo $($p.ExitCode)"
} else {
    Write-Diag "   OK: app viva (PID $($p.Id))"
}

Write-Diag "4) Ventanas visibles en el escritorio:"
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$root = [System.Windows.Automation.AutomationElement]::RootElement
$wins = $root.FindAll([System.Windows.Automation.TreeScope]::Children,
    [System.Windows.Automation.Condition]::TrueCondition)
foreach ($w in $wins) {
    $name = $w.Current.Name
    if ($name) { Write-Diag ("   Ventana: '{0}'" -f $name) }
}
if ($wins.Count -eq 0) { Write-Diag "   (sin ventanas detectadas)" }

Write-Diag "5) Buscando texto 'Actualizaci*disponible' (cubre con y sin tilde):"
$cond = New-Object System.Windows.Automation.PropertyCondition(
    [System.Windows.Automation.AutomationElement]::NameProperty, '*Actualizaci*disponible*')
$found = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $cond)
Write-Diag ("   coincidencias: {0}" -f $found.Count)

Write-Diag "=== Fin del diagnostico ==="
# No cerramos la app: que el usuario la vea en pantalla
