# Opera el selector de archivos NATIVO de Windows (IFileDialog, clase #32770)
# con mensajes Win32: escribe la ruta en el cuadro "Nombre de archivo" con
# WM_SETTEXT y pulsa el botón principal (IDOK). Sirve para "Abrir" y "Guardar".
# La prueba E2E no puede tocar ventanas nativas; este script corre en paralelo.
param(
    [Parameter(Mandatory = $true)][string]$Ruta,
    [int]$Plazo = 90,
    [string]$Log = "$env:TEMP\e2e-dialogo.log"
)

Add-Type @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public static class W32 {
    public delegate bool EnumProc(IntPtr h, IntPtr l);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr FindWindow(string cls, string title);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr p, EnumProc f, IntPtr l);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll")] public static extern int GetDlgCtrlID(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr GetParent(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr GetDlgItem(IntPtr d, int id);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr SendMessage(IntPtr h, uint m, IntPtr w, string l);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);

    public static string Clase(IntPtr h) { var s = new StringBuilder(256); GetClassName(h, s, 256); return s.ToString(); }
    public static string Texto(IntPtr h) { var s = new StringBuilder(512); GetWindowText(h, s, 512); return s.ToString(); }
    public static List<IntPtr> Hijos(IntPtr p) {
        var l = new List<IntPtr>();
        EnumChildWindows(p, (h, x) => { l.Add(h); return true; }, IntPtr.Zero);
        return l;
    }
    // Edit visible cuyo ancestro es el combo del nombre de archivo
    // (id 1148 en "Abrir", 1001 en "Guardar"); si no, el primer Edit visible.
    public static IntPtr CuadroNombre(IntPtr dlg) {
        IntPtr primero = IntPtr.Zero;
        foreach (var h in Hijos(dlg)) {
            if (Clase(h) != "Edit" || !IsWindowVisible(h)) continue;
            if (primero == IntPtr.Zero) primero = h;
            for (var a = GetParent(h); a != IntPtr.Zero && a != dlg; a = GetParent(a)) {
                int id = GetDlgCtrlID(a);
                if (id == 1148 || id == 1001) return h;
            }
        }
        return primero;
    }
}
"@

function Anota([string]$m) {
    $l = "[{0}] {1}" -f (Get-Date -Format "HH:mm:ss.fff"), $m
    Write-Output $l
    Add-Content -Path $Log -Value $l
}

function Selector {
    $h = [W32]::FindWindow("#32770", [NullString]::Value)
    if ($h -ne [IntPtr]::Zero -and [W32]::IsWindowVisible($h)) { return $h }
    return [IntPtr]::Zero
}

$WM_SETTEXT = 0x000C
$WM_COMMAND = 0x0111

# Bucle: mientras haya un selector visible, escribe la ruta y pulsa IDOK.
# Windows puede reconstruir el selector al abrirlo, así que cada intento
# vuelve a buscar la ventana y su cuadro de nombre.
Anota "esperando el selector nativo (ruta: $Ruta)"
$fin = (Get-Date).AddSeconds($Plazo)
$visto = $false
$hecho = $false
while ((Get-Date) -lt $fin) {
    $dlg = Selector
    if ($dlg -eq [IntPtr]::Zero) {
        if ($visto) { $hecho = $true; break }
        Start-Sleep -Milliseconds 250
        continue
    }
    $edit = [W32]::CuadroNombre($dlg)
    if ($edit -eq [IntPtr]::Zero) { Start-Sleep -Milliseconds 300; continue }
    if (-not $visto) {
        Anota ("selector listo: hwnd={0} título='{1}'" -f $dlg, [W32]::Texto($dlg))
        Start-Sleep -Milliseconds 700  # que termine de construirse
        $edit = [W32]::CuadroNombre($dlg)
    }
    $visto = $true
    [W32]::SendMessage($edit, $WM_SETTEXT, [IntPtr]::Zero, $Ruta) | Out-Null
    Anota ("ruta escrita (combo id={0}, texto ahora '{1}')" -f [W32]::GetDlgCtrlID([W32]::GetParent($edit)), [W32]::Texto($edit))
    # IDOK = 1: equivale a pulsar "Abrir"/"Guardar".
    [W32]::PostMessage($dlg, $WM_COMMAND, [IntPtr]1, [W32]::GetDlgItem($dlg, 1)) | Out-Null
    Anota "IDOK enviado"
    Start-Sleep -Milliseconds 1500
}
Anota ("selector cerrado: {0}" -f $hecho)
if (-not $hecho) { exit 2 }
