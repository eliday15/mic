# Prueba E2E de "Importar desde Access…" en la app MIC REAL sobre Windows,
# operada SOLO con UI Automation (el árbol accesible de WebView2): el mismo .exe
# de release que recibe el usuario, sin DevTools ni drivers.
#
# Recorre: bienvenida → "Importar desde Access…" → "Examinar…" → selector
# nativo (dialogo.ps1) → espera a que la inspección termine o falle. Deja
# capturas del escritorio, el texto del diálogo cada segundo, la salida del
# backend y la bitácora %TEMP%\mic-migracion.log en la carpeta de salida.
param(
    [Parameter(Mandatory = $true)][string]$App,
    [Parameter(Mandatory = $true)][string]$Mdb,
    [Parameter(Mandatory = $true)][string]$Salida,
    [int]$PlazoInspeccion = 240
)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
New-Item -ItemType Directory -Force -Path $Salida | Out-Null
$aqui = $PSScriptRoot
$AE = [System.Windows.Automation.AutomationElement]
$TS = [System.Windows.Automation.TreeScope]
$CT = [System.Windows.Automation.ControlType]
$t0 = Get-Date
$registro = Join-Path $Salida "e2e.log"

function Anota([string]$m) {
    $l = "[+{0:N1}s] {1}" -f ((Get-Date) - $t0).TotalSeconds, $m
    Write-Output $l
    Add-Content -Path $registro -Value $l
}

$script:n = 0
function Captura([string]$etq) {
    $script:n++
    $f = Join-Path $Salida ("{0:D2}-{1}.png" -f $script:n, $etq)
    try { & pwsh -NoProfile -File (Join-Path $aqui "captura.ps1") -Salida $f } catch { Anota "captura falló: $_" }
}

function Ventana($proc) {
    for ($i = 0; $i -lt 120; $i++) {
        $proc.Refresh()
        if ($proc.MainWindowHandle -ne [IntPtr]::Zero) { return $AE::FromHandle($proc.MainWindowHandle) }
        Start-Sleep -Milliseconds 500
    }
    throw "la app nunca mostró su ventana"
}

function Boton($raiz, [string]$texto, [int]$plazo = 60) {
    $cond = New-Object System.Windows.Automation.PropertyCondition($AE::ControlTypeProperty, $CT::Button)
    $fin = (Get-Date).AddSeconds($plazo)
    while ((Get-Date) -lt $fin) {
        foreach ($b in $raiz.FindAll($TS::Descendants, $cond)) {
            if ($b.Current.Name -like "*$texto*") { return $b }
        }
        Start-Sleep -Milliseconds 500
    }
    throw "no apareció el botón '$texto'"
}

function Invocar($el) {
    $el.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
}

# Texto visible del diálogo modal (role=dialog → ControlType Window/Pane con nombre).
function TextoModal($raiz) {
    $condDlg = New-Object System.Windows.Automation.PropertyCondition($AE::NameProperty, "Importar desde Access")
    $dlg = $raiz.FindFirst($TS::Descendants, $condDlg)
    if ($null -eq $dlg) { return "(sin modal)" }
    $nombres = @()
    foreach ($e in $dlg.FindAll($TS::Descendants, [System.Windows.Automation.Condition]::TrueCondition)) {
        $nm = $e.Current.Name
        if ($nm -and $nombres[-1] -ne $nm) { $nombres += $nm }
    }
    return ($nombres -join " | ")
}

$bitacora = Join-Path $env:TEMP "mic-migracion.log"
Remove-Item $bitacora -ErrorAction SilentlyContinue

Anota "app: $App"
Anota "mdb: $Mdb"
$env:RUST_LOG = "info"
$proc = Start-Process -FilePath $App -PassThru `
    -RedirectStandardOutput (Join-Path $Salida "backend.out") `
    -RedirectStandardError (Join-Path $Salida "backend.err")

$resultado = "desconocido"
try {
    $win = Ventana $proc
    Anota ("ventana: '{0}'" -f $win.Current.Name)
    $imp = Boton $win "Importar desde Access"
    Anota "app lista; invocando 'Importar desde Access…'"
    Invocar $imp

    $exa = Boton $win "Examinar"
    Captura "dialogo-abierto"
    Anota ("modal: {0}" -f (TextoModal $win))

    # El selector nativo lo opera otro proceso: Invoke puede no volver mientras
    # el selector modal está abierto.
    $helper = Start-Process pwsh -PassThru -NoNewWindow `
        -ArgumentList "-NoProfile", "-File", (Join-Path $aqui "dialogo.ps1"), "-Ruta", "`"$Mdb`"", "-Log", (Join-Path $Salida "dialogo.log")
    Anota "invocando 'Examinar…'"
    $job = Start-ThreadJob -ScriptBlock {
        param($h)
        Add-Type -AssemblyName UIAutomationClient
        $el = [System.Windows.Automation.AutomationElement]::FromHandle([IntPtr]$h)
        $c = New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
            [System.Windows.Automation.ControlType]::Button)
        foreach ($b in $el.FindAll([System.Windows.Automation.TreeScope]::Descendants, $c)) {
            if ($b.Current.Name -like "*Examinar*") {
                $b.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
                return "invocado"
            }
        }
        return "no encontrado"
    } -ArgumentList $proc.MainWindowHandle.ToInt64()

    $fin = (Get-Date).AddSeconds($PlazoInspeccion)
    $ultimo = ""
    $siguienteCaptura = (Get-Date).AddSeconds(5)
    while ((Get-Date) -lt $fin) {
        if ($proc.HasExited) { $resultado = "LA APP SE CERRÓ (código $($proc.ExitCode))"; break }
        try { $txt = TextoModal $win } catch { $txt = "(UIA no respondió: $_)" }
        if ($txt -ne $ultimo) { Anota "modal: $txt"; $ultimo = $txt }
        if ($txt -like "*Registros estimados*") { $resultado = "exito"; break }
        if ($txt -match "no se pudo|tardó más|error|Error") { $resultado = "error-visible"; break }
        if ((Get-Date) -gt $siguienteCaptura) { Captura "esperando"; $siguienteCaptura = (Get-Date).AddSeconds(20) }
        Start-Sleep -Seconds 1
    }
    if ($resultado -eq "desconocido") { $resultado = "COLGADO" }
    Anota ("job Examinar: {0} / {1}" -f $job.State, (Receive-Job $job -ErrorAction SilentlyContinue))
    Captura "final"
} catch {
    $resultado = "fallo-prueba: $_"
    Captura "fallo-prueba"
} finally {
    Anota "RESULTADO: $resultado"
    $contenido = if (Test-Path $bitacora) { Get-Content $bitacora -Raw } else { "(no existe)" }
    Write-Output "----- $bitacora -----"
    Write-Output $contenido
    Set-Content -Path (Join-Path $Salida "mic-migracion.log") -Value $contenido
    Set-Content -Path (Join-Path $Salida "resultado.txt") -Value $resultado
    if (-not $proc.HasExited) { Stop-Process -Id $proc.Id -Force }
}
if ($resultado -eq "exito") { exit 0 } else { exit 1 }
