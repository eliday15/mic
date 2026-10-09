# Opera el selector de archivos NATIVO de Windows (IFileOpenDialog, clase
# #32770) con UI Automation: espera a que aparezca, escribe la ruta en el cuadro
# "Nombre de archivo" y pulsa "Abrir". WebDriver no puede tocar ventanas
# nativas; este script corre en paralelo a la prueba E2E.
param(
    [Parameter(Mandatory = $true)][string]$Ruta,
    [int]$Plazo = 90,
    [string]$Log = "$env:TEMP\e2e-dialogo.log"
)

Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class W32 {
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern IntPtr FindWindow(string cls, string title);
    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")]
    public static extern bool IsWindowVisible(IntPtr h);
}
"@

function Anota([string]$m) {
    $l = "[{0}] {1}" -f (Get-Date -Format "HH:mm:ss.fff"), $m
    Write-Output $l
    Add-Content -Path $Log -Value $l
}

$AE = [System.Windows.Automation.AutomationElement]
$CTE = [System.Windows.Automation.ControlType]::Edit

# Selector visible (clase #32770) o IntPtr.Zero.
function Selector {
    $h = [W32]::FindWindow("#32770", [NullString]::Value)
    if ($h -ne [IntPtr]::Zero -and [W32]::IsWindowVisible($h)) { return $h }
    return [IntPtr]::Zero
}

# Cuadro del nombre de archivo: en "Abrir" es el Edit 1148 (dentro de un
# combo); en "Guardar" es el Edit 1001. Se llaman "File name:"/"Nombre de
# archivo:". Si el selector aún se está construyendo, no aparece todavía.
function CuadroNombre($dlg) {
    $edits = $dlg.FindAll([System.Windows.Automation.TreeScope]::Descendants,
        (New-Object System.Windows.Automation.PropertyCondition($AE::ControlTypeProperty, $CTE)))
    foreach ($e in $edits) {
        $id = $e.Current.AutomationId; $nm = $e.Current.Name
        if ($id -in @("1148", "1001") -or $nm -like "File name*" -or $nm -like "Nombre de archivo*") { return $e }
    }
    return $null
}

# Bucle: mientras haya un selector visible, intenta escribir la ruta y pulsar
# el botón principal (id 1). Windows puede reconstruir el selector al abrirlo,
# así que cada intento vuelve a buscar la ventana.
Anota "esperando el selector nativo (ruta: $Ruta)"
$fin = (Get-Date).AddSeconds($Plazo)
$visto = $false
$hecho = $false
while ((Get-Date) -lt $fin) {
    $hwnd = Selector
    if ($hwnd -eq [IntPtr]::Zero) {
        if ($visto) { $hecho = $true; break }
        Start-Sleep -Milliseconds 250
        continue
    }
    try {
        $dlg = $AE::FromHandle($hwnd)
        $edit = CuadroNombre $dlg
        if ($null -eq $edit) { Start-Sleep -Milliseconds 300; continue }
        if (-not $visto) { Anota ("selector listo: hwnd=$hwnd título='{0}'" -f $dlg.Current.Name) }
        $visto = $true
        $edit.SetFocus()
        $edit.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).SetValue($Ruta)
        Anota ("ruta escrita en el edit id='{0}'" -f $edit.Current.AutomationId)
        $condBtn = New-Object System.Windows.Automation.AndCondition(
            (New-Object System.Windows.Automation.PropertyCondition($AE::AutomationIdProperty, "1")),
            (New-Object System.Windows.Automation.PropertyCondition($AE::ControlTypeProperty, [System.Windows.Automation.ControlType]::Button))
        )
        $btn = $dlg.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $condBtn)
        if ($null -ne $btn) {
            $btn.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
            Anota ("botón '{0}' invocado" -f $btn.Current.Name)
        } else {
            Anota "sin botón id=1: se envía Enter"
            [W32]::SetForegroundWindow($hwnd) | Out-Null
            Add-Type -AssemblyName System.Windows.Forms
            [System.Windows.Forms.SendKeys]::SendWait("{ENTER}")
        }
    } catch {
        Anota "reintento tras error UIA: $_"
    }
    Start-Sleep -Milliseconds 1500
}
Anota ("selector cerrado: {0}" -f $hecho)
if (-not $hecho) { exit 2 }
