# Prueba E2E de "Importar desde Access…" en la app MIC REAL sobre Windows,
# operada SOLO con UI Automation (el árbol accesible de WebView2): el mismo .exe
# de release que recibe el usuario, sin DevTools ni drivers.
#
# Recorre la importación COMPLETA: bienvenida → "Importar desde Access…" →
# selector nativo del .mdb (dialogo.ps1) → inspección → selector de destino →
# migración → reporte → abrir el álbum. Deja
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
    # UIA entre procesos puede fallar de forma transitoria mientras hay un
    # selector modal abierto: se reintenta en vez de abortar la prueba.
    for ($i = 0; $i -lt 5; $i++) {
        try {
            $condDlg = New-Object System.Windows.Automation.PropertyCondition($AE::NameProperty, "Importar desde Access")
            $dlg = $raiz.FindFirst($TS::Descendants, $condDlg)
            if ($null -eq $dlg) { return "(sin modal)" }
            $nombres = @()
            foreach ($e in $dlg.FindAll($TS::Descendants, [System.Windows.Automation.Condition]::TrueCondition)) {
                $nm = $e.Current.Name
                if ($e.Current.ControlType -eq $CT::Edit) {
                    $vp = $null
                    if ($e.TryGetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern, [ref]$vp)) {
                        $nm = "$nm=[$($vp.Current.Value)]"
                    }
                }
                if ($nm -and $nombres[-1] -ne $nm) { $nombres += $nm }
            }
            return ($nombres -join " | ")
        } catch {
            Start-Sleep -Milliseconds 500
        }
    }
    return "(UIA no respondió)"
}

$bitacora = Join-Path $env:TEMP "mic-migracion.log"
Remove-Item $bitacora -ErrorAction SilentlyContinue

Anota "app: $App"
Anota "mdb: $Mdb"
$env:RUST_LOG = "info"
$proc = Start-Process -FilePath $App -PassThru `
    -RedirectStandardOutput (Join-Path $Salida "backend.out") `
    -RedirectStandardError (Join-Path $Salida "backend.err")

# Invoca un botón que abre un selector NATIVO y lo opera con dialogo.ps1.
# El Invoke va en un hilo aparte: puede no volver mientras el selector modal
# está abierto.
function ClicConSelector($proc, [string]$boton, [string]$ruta, [string]$etq) {
    Start-Process pwsh -NoNewWindow -ArgumentList "-NoProfile", "-File", (Join-Path $aqui "dialogo.ps1"), `
        "-Ruta", "`"$ruta`"", "-Log", (Join-Path $Salida "dialogo-$etq.log") | Out-Null
    Anota "invocando '$boton' (selector: $ruta)"
    Start-ThreadJob -ScriptBlock {
        param($h, $texto)
        Add-Type -AssemblyName UIAutomationClient
        $el = [System.Windows.Automation.AutomationElement]::FromHandle([IntPtr]$h)
        $c = New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
            [System.Windows.Automation.ControlType]::Button)
        foreach ($b in $el.FindAll([System.Windows.Automation.TreeScope]::Descendants, $c)) {
            if ($b.Current.Name -like "*$texto*") {
                $b.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
                return "invocado"
            }
        }
        return "no encontrado"
    } -ArgumentList $proc.MainWindowHandle.ToInt64(), $boton | Out-Null
}

# Espera a que el texto del diálogo contenga $exito (→ $true) o un error
# visible (→ excepción). Captura el escritorio cada 20 s mientras espera.
function Esperar($win, $proc, [string]$exito, [string]$etq, [int]$plazo = $PlazoInspeccion) {
    $fin = (Get-Date).AddSeconds($plazo)
    $ultimo = ""
    $siguienteCaptura = (Get-Date).AddSeconds(5)
    while ((Get-Date) -lt $fin) {
        if ($proc.HasExited) { throw "LA APP SE CERRÓ (código $($proc.ExitCode))" }
        try { $txt = TextoModal $win } catch { $txt = "(UIA no respondió: $_)" }
        if ($txt -ne $ultimo) { Anota "modal: $txt"; $ultimo = $txt }
        if ($txt -like "*$exito*") { Captura "ok-$etq"; return }
        if ($txt -match "no se pudo|tardó más|Error") { Captura "error-$etq"; throw "error visible en '$etq': $txt" }
        if ((Get-Date) -gt $siguienteCaptura) { Captura "esperando-$etq"; $siguienteCaptura = (Get-Date).AddSeconds(20) }
        Start-Sleep -Seconds 1
    }
    Captura "colgado-$etq"
    throw "COLGADO en '$etq': la interfaz nunca mostró '$exito' en $plazo s (último: $ultimo)"
}

$resultado = "desconocido"
try {
    $win = Ventana $proc
    Anota ("ventana: '{0}'" -f $win.Current.Name)
    $imp = Boton $win "Importar desde Access"
    Anota "app lista; invocando 'Importar desde Access…'"
    Invocar $imp
    Boton $win "Examinar" | Out-Null
    Captura "dialogo-abierto"

    # 1) Elegir el .mdb → inspección.
    ClicConSelector $proc "Examinar" $Mdb "origen"
    Esperar $win $proc "Registros estimados" "inspeccion"

    # 2) Elegir destino (selector de guardar) → ejecutar migración.
    $destino = Join-Path $env:TEMP ("e2e-{0}.micdb" -f [IO.Path]::GetFileNameWithoutExtension($Mdb))
    Remove-Item $destino -ErrorAction SilentlyContinue
    ClicConSelector $proc "Examinar" $destino "destino"
    $fin = (Get-Date).AddSeconds(60)
    while ((Get-Date) -lt $fin -and -not ((TextoModal $win) -like "*$destino*")) {
        if ($proc.HasExited) { throw "LA APP SE CERRÓ con el selector de destino (código $($proc.ExitCode))" }
        Start-Sleep -Milliseconds 500
    }
    if (-not ((TextoModal $win) -like "*$destino*")) { Captura "sin-destino"; throw "el destino nunca llegó al diálogo" }
    Anota ("modal: {0}" -f (TextoModal $win))
    Invocar (Boton $win "Ejecutar migración")
    Esperar $win $proc "Registros principales" "migracion"
    if (-not (Test-Path $destino)) { throw "la migración dijo éxito pero no existe $destino" }
    Anota ("álbum creado: {0} ({1:N0} bytes)" -f $destino, (Get-Item $destino).Length)

    # 3) Abrir el álbum migrado: el diálogo se cierra y la bienvenida desaparece.
    $condBtn = New-Object System.Windows.Automation.PropertyCondition($AE::ControlTypeProperty, $CT::Button)
    $abrir = $null
    foreach ($b in $win.FindAll($TS::Descendants, $condBtn)) { if ($b.Current.Name -eq "Abrir") { $abrir = $b } }
    if ($null -eq $abrir) { throw "no apareció el botón 'Abrir' del reporte" }
    Invocar $abrir
    $fin = (Get-Date).AddSeconds(60)
    $abierto = $false
    while ((Get-Date) -lt $fin) {
        $bienvenida = $false
        foreach ($b in $win.FindAll($TS::Descendants, $condBtn)) { if ($b.Current.Name -like "*Importar desde Access*") { $bienvenida = $true } }
        if ((TextoModal $win) -eq "(sin modal)" -and -not $bienvenida) { $abierto = $true; break }
        Start-Sleep -Milliseconds 500
    }
    Captura "album-abierto"
    if (-not $abierto) { throw "el álbum migrado no se abrió" }
    $resultado = "exito"
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
