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
}
"@

function Anota([string]$m) {
    $l = "[{0}] {1}" -f (Get-Date -Format "HH:mm:ss.fff"), $m
    Write-Output $l
    Add-Content -Path $Log -Value $l
}

$AE = [System.Windows.Automation.AutomationElement]
$fin = (Get-Date).AddSeconds($Plazo)
Anota "esperando el selector nativo (ruta: $Ruta)"
$hwnd = [IntPtr]::Zero
while ((Get-Date) -lt $fin) {
    $hwnd = [W32]::FindWindow("#32770", [NullString]::Value)
    if ($hwnd -ne [IntPtr]::Zero) { break }
    Start-Sleep -Milliseconds 250
}
if ($hwnd -eq [IntPtr]::Zero) { Anota "ERROR: el selector nunca apareció"; exit 2 }
Anota "selector encontrado: hwnd=$hwnd"
Start-Sleep -Milliseconds 800

$dlg = $AE::FromHandle($hwnd)
Anota ("título del selector: '{0}'" -f $dlg.Current.Name)

# Cuadro "Nombre de archivo": AutomationId 1148 (combo) → su Edit interno.
$condEdit = New-Object System.Windows.Automation.AndCondition(
    (New-Object System.Windows.Automation.PropertyCondition($AE::AutomationIdProperty, "1148")),
    (New-Object System.Windows.Automation.PropertyCondition($AE::ControlTypeProperty, [System.Windows.Automation.ControlType]::Edit))
)
$edit = $dlg.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $condEdit)
if ($null -ne $edit) {
    $vp = $edit.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)
    $vp.SetValue($Ruta)
    Anota "ruta escrita con ValuePattern"
    $condBtn = New-Object System.Windows.Automation.AndCondition(
        (New-Object System.Windows.Automation.PropertyCondition($AE::AutomationIdProperty, "1")),
        (New-Object System.Windows.Automation.PropertyCondition($AE::ControlTypeProperty, [System.Windows.Automation.ControlType]::Button))
    )
    $btn = $dlg.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $condBtn)
    if ($null -ne $btn) {
        $ip = $btn.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
        $ip.Invoke()
        Anota "botón Abrir invocado"
    } else {
        Anota "sin botón Abrir: se envía Enter"
        [W32]::SetForegroundWindow($hwnd) | Out-Null
        Add-Type -AssemblyName System.Windows.Forms
        [System.Windows.Forms.SendKeys]::SendWait("{ENTER}")
    }
} else {
    Anota "sin cuadro 1148: se teclea la ruta + Enter"
    [W32]::SetForegroundWindow($hwnd) | Out-Null
    Add-Type -AssemblyName System.Windows.Forms
    [System.Windows.Forms.SendKeys]::SendWait($Ruta)
    [System.Windows.Forms.SendKeys]::SendWait("{ENTER}")
}

# ¿Se cerró el selector?
$cerro = $false
for ($i = 0; $i -lt 40; $i++) {
    Start-Sleep -Milliseconds 250
    if ([W32]::FindWindow("#32770", [NullString]::Value) -eq [IntPtr]::Zero) { $cerro = $true; break }
}
Anota ("selector cerrado: {0}" -f $cerro)
