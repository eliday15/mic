# Crea un album MIC en formato Jet 3.x (Access 95/97) con el motor Jet REAL
# de Microsoft, igual que lo hacia el VB6 (CreateDatabase ... dbVersion30).
# Requiere PowerShell de 32 bits (msjet40.dll es solo 32-bit).
$ErrorActionPreference = "Stop"
$p = Join-Path $env:TEMP "mic-jet3.mdb"
if (Test-Path $p) { Remove-Item $p -Force }

$cat = New-Object -ComObject ADOX.Catalog
# Engine Type=4 -> formato Jet 3.x (el mismo que dbVersion30 de DAO)
$cat.Create("Provider=Microsoft.Jet.OLEDB.4.0;Data Source=$p;Jet OLEDB:Engine Type=4") | Out-Null

$cn = New-Object -ComObject ADODB.Connection
$cn.Open("Provider=Microsoft.Jet.OLEDB.4.0;Data Source=$p")

$cn.Execute("CREATE TABLE propiedades (Nombre TEXT(50), Tipo BYTE, longitud BYTE, decimales BYTE, totalizable BIT, sInfo TEXT(100), TipoSal BYTE, Enprincipal BIT, Modificable BIT, Visible BIT, OrdenVisible BYTE)") | Out-Null
$props = @(
  "('_imagen_',0,255,0,0,' ',0,1,1,0,0)", "('_id_',1,0,0,0,' ',0,1,0,0,0)",
  "('_auxiliar_',1,0,0,0,' ',0,1,0,0,0)", "('_variantes_',1,0,0,0,' ',0,1,0,0,0)",
  "('Clave',0,50,0,0,' ',0,1,1,1,1)", "('Descripción',0,255,0,0,' ',0,1,1,1,2)",
  "('Cantidad',1,10,0,1,' ',0,1,1,1,3)", "('Precio',2,10,2,1,' ',0,1,1,1,4)",
  "('Fecha Alta',3,0,0,0,' ',0,1,1,1,5)", "('Importe',4,0,2,1,'Cantidad*Precio',1,1,0,1,6)",
  "('Etiquetas',5,0,0,0,' ',0,1,1,1,7)", "('Talla',0,20,0,0,' ',0,0,1,1,1)",
  "('Color',0,30,0,0,' ',0,0,1,0,0)")
foreach ($v in $props) { $cn.Execute("INSERT INTO propiedades VALUES $v") | Out-Null }

$cn.Execute("CREATE TABLE Principal ([Clave] TEXT(50), [Descripción] MEMO, [Cantidad] DOUBLE, [Precio] CURRENCY, [Fecha Alta] DATETIME, [Importe] DOUBLE, [Etiquetas] TEXT(50), [_imagen_] TEXT(255), [_id_] LONG, [_auxiliar_] BIT, [_variantes_] BIT)") | Out-Null
$descs = @("Cinturón de piel genuina","Camisa mañanera de algodón","Pantalón añejo estilo español","Bolsa de diseño con corazón","Zapato niño")
for ($i = 1; $i -le 60; $i++) {
  $d = $descs[$i % 5] + " número $i — año " + (2000 + ($i % 8))
  $conVar = if ($i % 5 -eq 1) { 1 } else { 0 }
  $fecha = "#{0:MM/dd/yyyy}#" -f (Get-Date -Year (2000 + ($i % 20)) -Month (1 + ($i % 12)) -Day (1 + ($i % 27)))
  $cant = $i * 3; $precio = $i * 17.5; $importe = $cant * $precio
  $sql = "INSERT INTO Principal VALUES ('ALM-{0:d4}','{1}',{2},{3},{4},{5},NULL,'G:\MIC\imagenes\alm$i.jpg',$i,0,$conVar)" -f $i, $d.Replace("'","''"), $cant, $precio, $fecha, $importe
  $cn.Execute($sql) | Out-Null
}

$cn.Execute("CREATE TABLE Variantes ([Talla] TEXT(20), [Color] TEXT(30), [_imagen_] TEXT(255), [_id_] LONG, [_idprincipal_] LONG)") | Out-Null
$vid = 1000
for ($i = 1; $i -le 60; $i += 5) {
  $cn.Execute("INSERT INTO Variantes VALUES ('CH','Añil','G:\MIC\imagenes\v$vid.jpg',$vid,$i)") | Out-Null; $vid++
  $cn.Execute("INSERT INTO Variantes VALUES ('XG','Café','G:\MIC\imagenes\v$vid.jpg',$vid,$i)") | Out-Null; $vid++
}

$cn.Execute("CREATE TABLE Multidatos ([Id] LONG, [Principal] BIT, [Campo_n] TEXT(50), [Valor] TEXT(255))") | Out-Null
for ($i = 1; $i -le 60; $i += 3) {
  $cn.Execute("INSERT INTO Multidatos VALUES ($i,1,'Etiquetas','promoción')") | Out-Null
  $cn.Execute("INSERT INTO Multidatos VALUES ($i,1,'Etiquetas','línea clásica')") | Out-Null
}

$cn.Execute("CREATE TABLE Categorias ([Campo_n] TEXT(50), [Principal] BIT, [Valor] TEXT(255), [Default] BIT)") | Out-Null
$cn.Execute("INSERT INTO Categorias VALUES ('Etiquetas',1,'promoción',1)") | Out-Null
$cn.Execute("INSERT INTO Categorias VALUES ('Etiquetas',1,'línea clásica',0)") | Out-Null

$cn.Close()
Copy-Item $p "mic-jet3.mdb" -Force
Write-Host "creado: mic-jet3.mdb ($((Get-Item mic-jet3.mdb).Length) bytes) formato Jet 3.x"
