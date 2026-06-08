# INVENTARIO FUNCIONAL COMPLETO - APLICACIÓN MIC
## Gestor de Álbumes de Imágenes con Fichas Catalográficas (VB6, Access .mdb)

---

## 1. MENÚ PRINCIPAL (frmMain)

### 1.1 Menú "&Archivo"
- **&Nuevo Album...** (Ctrl+N) - Abre frmNuevo para crear álbum desde plantilla o nuevo
- **Nueva &Imagen** (Ctrl+I, deshabilitado sin álbum abierto) - Añade imagen al álbum actual
- **&Abrir...** (Ctrl+O) - Abre selector para cargar álbum .mdb existente (clsSeleccion)
- **&Usar Respaldo** (Ctrl+R) - Recupera álbum desde respaldo (deshabilitado)
- **&Cerrar** - Cierra álbum actual (deshabilitado sin álbum)
- **&Guardar** - Persiste cambios del álbum actual (deshabilitado sin álbum)
- **C&opiar Album...** - Abre frmCopAlbum/Dialog para duplicar álbum (solo estructura o con datos)
- **Exportar...** - Abre frmExp para exportar registros a otros formatos (selectivos por campo)
- **Importar...** - ActConExterno en documento actual (deshabilitado)
- **&Imprimir...** (deshabilitado sin álbum) - Abre frmprint (reporte con imágenes)
- **ReporteSinImg...** (deshabilitado sin álbum) - Abre frmprint2 (tabla de datos sin imágenes)
- **&Salir** - Cierra aplicación

### 1.2 Menú "&Editar" (visible solo con álbum abierto)
- **&Undo** - Deshacer (no implementado)
- **Cu&t** (Ctrl+X) - Cortar en RTF
- **&Copy** (Ctrl+C) - Copiar en RTF
- **&Paste** (Ctrl+V) - Pegar en RTF
- **Paste &Special...** - Pegar especial (no implementado)
- **Ca&mpos a la Vista** - Abre frmDALV para configurar campos visibles/modificables
- **Selecciona Todos** - Selecciona todas las imágenes en página
- **InvierteSeleccion** - Invierte selección de imágenes

### 1.3 Menú "&Ver" (visible solo con álbum abierto)
- **&Toolbar** (checked) - Muestra/oculta barra de herramientas
- **Status &Bar** (checked) - Muestra/oculta barra de estado
- **&Refresh** - Recarga datos (no implementado)
- **&Options...** - Configuración (no implementado)
- **&Web Browser** - Abre navegador (no implementado)

### 1.4 Menú "&Herramientas"
- **&Ordenar...** (deshabilitado sin álbum) - Abre frmOrdenar para multi-nivel sort (hasta 3 campos + agrupación)
- **Act. Gral. de &Datos...** (deshabilitado sin álbum) - Abre frmActGrlDat para actualizar masivamente valores
- **Directorios** (submenú)
  - **&Imagenes...** - Selecciona directorio de imágenes
  - **&Albums...** - Selecciona directorio de álbumes
  - **&Plantillas...** - Selecciona directorio de plantillas
  - **&EmpacadosImportados** - Selecciona directorio de paquetes
- **Albums &Ligados...** (deshabilitado sin álbum) - Abre frmAlbumsL para gestionar álbumes relacionados
- **Actualizar Ligas** (oculto) - Sincroniza datos entre álbumes ligados
- **Importar...** (oculto, deshabilitado sin álbum) - ActConExterno
- **EmpacarAlbum...** (deshabilitado sin álbum) - Comprime álbum completo (.mdb + imágenes)
- **Desempacar...** - Abre frm3Botones para expandir paquete comprimido
- **Asociar el mic a .mdb** - Registro Windows: vincula extensión .mdb a MIC
- **Totalizar** (deshabilitado sin álbum) - Abre frmTotalizar para sumatorias por campo
- **Modificar Album** - Abre frmnewp para editar estructura de álbum existente
- **Oculta/Muestra Panel** (deshabilitado sin álbum) - Contrae/expande panel lateral (Grupos/Filtros)
- **Act. Multidatos...** - ActualizaEstructura: sincroniza estructura base con álbumes
- **Desinstalar** - Genera código de desinstalación (protección anticopia)
- **Act. Calculados** (deshabilitado sin álbum) - Recalcula campos de fórmula
- **Imagenes de Dir** (deshabilitado sin álbum) - Anexar imágenes masivamente desde carpeta (NuevaImagen con hwnd)

### 1.5 Menú "&Window"
- **&Cascade** - Organiza ventanas en cascada
- **Tile &Horizontal** - Mosaico horizontal
- **Tile &Vertical** - Mosaico vertical
- **&Arrange Icons** - Organiza iconos minimizados
- **Listado dinámico** de ventanas abiertas (MDI)

### 1.6 Menú "&Ayuda"
- **&Contenido** - Abre archivo .hlp
- **&Search For Help On...** (oculto) - Búsqueda en help
- **&Acerca de...** - Abre frmAbout (información versión)
- **Registrar...** - Abre diálogo de registro (clave de protección)

### 1.7 Contexto "Imagen" (clic derecho en imagen)
- **&Nueva** - Añade imagen nueva (NuevaImagen)
- **&Editar** - Abre editor de imagen (EditaImagen)
- **&Eliminar** - Elimina imagen seleccionada (requiere selección)
- **&Ocultar** - Oculta imagen sin eliminarla
- **Ca&mpos a la Vista** - Configurar visibilidad de campos
- **&Ver Imagen 100%** - Visor a escala real (MuestraImagen)

### 1.8 Contexto "Multidato"
- **&Borrar** - Elimina valor de multidato
- **&Editar** - Modifica valor de multidato

### 1.9 Barra de Herramientas (Toolbar)
- **New** - Nuevo álbum
- **NewImage** (deshabilitado) - Nueva imagen
- **Open** - Abrir álbum
- **Separador**
- **OchoXLinea** (deshabilitado) - 8 imágenes por línea (thumbnail 8x)
- **CuatroXLinea** (deshabilitado) - 4 imágenes por línea (thumbnail 4x)
- **DosXLinea** (deshabilitado) - 2 imágenes por línea (thumbnail 2x)
- **Separador**
- **primera** (deshabilitado) - Primera página
- **anterior** (deshabilitado) - Página anterior
- **buscar** (deshabilitado) - Buscar (Ctrl+F)
- **siguiente** (deshabilitado) - Página siguiente
- **ultima** (deshabilitado) - Última página
- **Separador**
- **ordenar** (deshabilitado) - Ordenar registros
- **actgral** (deshabilitado) - Actualización general de datos
- **totaliza** (deshabilitado) - Totalizar

---

## 2. GESTIÓN DE ÁLBUMES

### 2.1 frmNuevo - Selección de Plantilla / Álbum Nuevo
**Propósito:** Wizard para crear álbum desde plantilla o plantilla predefinida
- **Vista:** Icono grande, lista, detalles
- **Funciones:**
  - Listado de plantillas (.xms) en directorio
  - Vista previa de estructura (campos)
  - Botón "Nuevo Album" - Crea album en blanco
  - Botón "Abrir" - Abre plantilla seleccionada en editor
  - Botón "Eliminar" - Borra plantilla

### 2.2 frmnewp - Editor de Plantilla / Estructura de Álbum
**Propósito:** Define/edita campos del álbum (tipo, longitud, propiedades)
- **Grid Principal:**
  - Columnas: # | Nombre | Tipo | Long | Dec | Tot(alizable)
  - Tipos disponibles: Texto, Numérico, Moneda, Fecha, Calculado, Multidato
- **Botones:**
  - **&Nuevo** - Abre frmauxcmp para crear campo
  - **&Editar** - Modifica campo existente
  - **&Borrar** - Elimina campo de grid
  - **&Variantes** - Abre variante de frmnewp para campos secundarios (opcional)
  - **Guardar &Plantilla** - Persiste como .xms
  - **&Guardar Album** - Crea .mdb con estructura
  - **Editar Formula** - frmCaptForm para campos calculados (fórmulas con otros campos)
- **Propiedades por tipo de campo:**
  - **Texto:** longitud (0-255 chars)
  - **Numérico/Moneda:** longitud total, decimales, formato
  - **Fecha:** formato dd/MM/yyyy
  - **Calculado:** fórmula + tipo salida (Numérico/Fecha)
  - **Multidato:** lista de valores (no editable desde ficha, gestiona en frmCaptura)
- **Variantes:** Permite sub-registros por imagen (ej. colores, tallas, variaciones)

### 2.3 frmEdCmps - Estructura de Álbum (Alternativa)
**Propósito:** Editor visual similar a frmnewp con radio buttons para tipo

### 2.4 frmauxcmp - Edición Auxiliar de Campo
**Propósito:** Ventana modal para definir un campo individual
- **Campos:**
  - Nombre (validado: sin duplicados)
  - Tipo (radio buttons)
  - Longitud (si aplica)
  - Decimales (numéricos/moneda)
  - Checkbox: Totalizable
  - Botón "Editar Formula" (solo para Calculado)

### 2.5 frmCaptForm - Editor de Fórmula
**Propósito:** Construye fórmula para campo calculado
- **Entrada:** Lista de campos disponibles (solo Numérico, Moneda, Fecha, Calculado)
- **Función:** Validar sintaxis, calcular ejemplo con valores = 10
- **Salida:** Fórmula + tipo de resultado (Numérico o Fecha)

---

## 3. CAPTURA Y EDICIÓN DE DATOS (frmDocument - Ventana MDI Principal)

### 3.1 Estructura General
- **Panel Izquierdo:** SSTab (2 pestañas)
  - Pestaña "Grupos" - TreeView + Combo de grupos, botones Nuevo/Borrar/Refresca
  - Pestaña "Filtros" - picfilt para configurar filtros
- **Panel Derecho (Scroller):** Grid de imágenes thumbnails
- **Barra Inferior:** StatusBar con 10 paneles (estado, ocultarPanel, rango, navegación, filtro, orden, refresco)
- **Menú Contexto:** Imagen (Nueva, Editar, Eliminar, Ocultar, Campos a Vista, Ver 100%)

### 3.2 Visualización de Imágenes
- **Escalas configurable:** 8, 4, 2 imágenes por línea (botones toolbar o menú)
- **Cada imagen:** Marco seleccionable, thumbnail con borde
- **Propiedades:** Tag contiene ruta de archivo imagen
- **Controles:**
  - Image1(índice) - array de imágenes
  - Marco(índice) - rectángulos de selección
  - txtv(índice) - textbox dinámico para campos editable
  - comEnter, Comaf, dtp, lstiws - controles para tipos de datos

### 3.3 Captura / Edición de Datos (frmCaptura)
**Propósito:** Formulario modal para editar datos de registro
- **Estructura:**
  - Label header: "Variantes." y "Edicion de campos."
  - Scroller con controles dinámicos por campo
  - Label1(i) - nombre de campo
  - Text1(i) - para Texto
  - lstiws(i) - para Multidato (listbox con entrada nueva)
  - dtp(i) - para Fecha (DTPicker)
  - comEnter(i) - botón OK para entrada
  - Comaf(i) - botón búsqueda asistida (Autofill)
- **Botones:** Aceptar, Cancelar
- **Lógica:** 
  - Si nuevo: pide imagen obligatoria (CapturaCampos → clsSeleccion)
  - Múltiples imágenes: icount > 1, crea registros asociados
  - Variantes: subformulario para datos alternativos

### 3.4 Autofill / Búsqueda Asistida (frmAutofill)
**Propósito:** Lista de valores predefinidos para campo (categorías)
- **Listview:** Listado de palabras guardadas
- **Botones:**
  - **Nuevo** - InputBox para añadir valor
  - **Editar** - Modifica valor seleccionado
  - **Borrar** - Elimina valor
  - **Default** - Marca valor como predeterminado (icono)
  - **OK** - Aplica valor seleccionado a campo
- **Fuente datos:** Tabla Categorias en .mdb (Campo_n, Principal/variante, Valor, Default)

### 3.5 Selección de Múltiples Imágenes (clsSeleccion)
**Propósito:** Diálogo de archivos reutilizable
- **Métodos:**
  - seleccionar(multiple, título, extensión) - Selecciona archivo/s
  - selArchivosImagenes(hwnd) - Para captura masiva desde directorio
- **Propiedades:**
  - archivo - ruta del archivo seleccionado
  - cantidad - número de archivos si múltiple

---

## 4. BÚSQUEDA Y FILTRADO

### 4.1 frmBuscar - Búsqueda por Texto/Valor
**Propósito:** Localiza registros por coincidencia en campos
- **Controles:**
  - TextBox: "Buscar:" (búsqueda incremental)
  - CheckBox: "Mayusculas/Minusculas" (modo exacto)
  - Botones: "Buscar", "Buscar Siguiente", "Cerrar"
- **Lógica de Búsqueda (BuscaEnDatosDeImagen):**
  - **Texto:** InStr case-sensitive/insensitive (configurable)
  - **Numérico/Moneda:** Exacto (elimina comas)
  - **Fecha:** CVDate exacto
  - **Calculado:** Según tipo salida
  - **Multidato:** Busca en tabla multidatos por Id + Campo_n
  - **Variantes:** Si existe registro principal sin coincidencia, busca en tabla variantes
- **Resultado:** 
  - Mensajes de estado (localizado/fin de archivo)
  - Selecciona imagen y actualiza página si es necesario

### 4.2 Filtros Avanzados (frmFA)
**Propósito:** Filtrado multi-criterio (AND/OR lógica)
- **Estructura:**
  - Hasta 3 filas de criterios
  - Cada fila: [Operador Relación] [Campo] [Operador Comparación] [Valor]
  - Operadores: AND, OR
  - Comparadores: =, <>, <, >, <=, >=, LIKE, IN
- **Botones:**
  - **Aceptar** - Aplica filtro
  - **Grabar** - Persiste configuración de filtro (tabla FiltrosAv)
  - **Abrir** - Carga filtro guardado
  - **Cancelar** - Descarta
  - **+/-** - Agrega/quita filas de criterio
- **Almacenamiento:** Tabla FiltrosAv (Nombre, OpR, Campo, OpC, Valor)

### 4.3 frmAbreFA - Selector de Filtros Guardados
**Propósito:** Carga filtro avanzado previamente guardado

### 4.4 frmFiltros - Dialog básico (stub)

### 4.5 frmOrdenar - Ordenamiento Multi-Nivel
**Propósito:** Define hasta 3 niveles de orden (ascendente/descendente)
- **Campos:**
  - Combo1, Combo2, Combo3 - Selecciona campo por nivel
  - opt1/2/3(0/1) - Ascendente/Descendente por nivel
- **Modo Grupo (iTipo = GR_NUEVO/GR_EDITADO):**
  - TextBox "Nombre de Agrupacion" - Nombre del grupo
  - Visualización cambia para agrupar + ordenar sub-registros
- **Resultado:** Ordena tabla en memoria, recalcula páginas

---

## 5. NAVEGACIÓN POR PÁGINAS

### 5.1 Paginación (clsPaginas)
**Propósito:** Gestiona visualización de registros en múltiples páginas
- **Propiedades:**
  - PaginaNum - Página actual
  - TotalPaginas - Total de páginas
  - TotalEnPag - Registros en página actual
  - ImagenIni - Índice de primer imagen en página
  - bmks() - Array de bookmarks DAO para acceso rápido
- **Métodos:**
  - Primera(tabla) - Va a primera página
  - Avanza(tabla) - Siguiente página
  - Retrocede(tabla) - Página anterior
  - Ultima(tabla) - Última página

### 5.2 Controles de Navegación
- **Toolbar:** Botones primera, anterior, siguiente, última
- **StatusBar:** Panel "rango en ventana/total" (ej. "1-8/42")
- **Atajo teclado:** Flechas, Re Pag, Av Pag (en frmDocument)

---

## 6. IMPRESIÓN Y REPORTES

### 6.1 frmprint - Imprimir con Imágenes
**Propósito:** Catálogo impreso: imágenes + datos + configuración de página
- **Controles Principales:**
  - cmbRep - Selector de reporte guardado (clase clsReporteCI)
  - msfg - Grid de configuración de campos (nombre, líneas x campo, incluir)
  - cmbprnt(2) - Selector de bandeja/dispositivo
  - Coms(4) - "Modificar -->" botón para editar reporte
- **Opciones de Reporte (clsReporteCI):**
  - Nombre - Identificador
  - HojaOficio - Verdadero=Oficio, Falso=Carta
  - Orientacion - 0=Vertical, 1=Horizontal
  - FontSize, FontSizeTitle - Tamaño fuentes
  - titulo - Encabezado
  - PonPagina, PonFecha - Incluir número página y fecha
  - PonTotales - Sumar campos totalizables
  - ImagenesXLinea - 1, 2, 4, 8 imágenes por fila
  - ImagenesXLineaV - Ídem para variantes
  - campos() - Lista de campos a imprimir
  - LineasXCampo() - Alto asignado por campo
  - IncluirCampo() - Bandera X/vacío
  - CamposV, LineasXCampoV - Variantes
- **Almacenamiento:** Tabla Reportes (clsReporteCI.GetReporte)
- **Funciones de Report:**
  - CargaGrid - Poblador de msfg
  - SetReporteNuevo - Inicializa reporte nuevo desde campos visibles
- **Botón "Configurar Pagina..."** - Cmdprn(6) ajustes impresora/márgenes

### 6.2 frmprint2 - Reporte Sin Imágenes (Tabla de Datos)
**Propósito:** Datos estructurados en tabla (texto plano o Excel-like)
- **Controles:**
  - cmdprn(0) - Agrupar (opcional)
  - Coms(4) - "Editar >" para configurar columnas
  - cmbprnt(2) - Dispositivo
- **Opciones (clsReporteSI):**
  - Nombre, HojaOficio, HojaHorizontal
  - FontSize, FontSizeTitle
  - titulo, PonPagina, PonFecha
  - Agrupacion - Campo para agrupar y totalizar
  - campos(), LineasXCampo()
  - caractXCampo() - Ancho de columna
  - totalizable() - Sumar estos campos
  - Encabezado() - Títulos de columna
- **Métodos:**
  - CargaGrid(msfg) - Llena grid de columnas
  - AlmacenaReporte - Persiste en tabla reportessi

### 6.3 frmPreliminar - Vista Preliminar
**Propósito:** Preview del documento antes de imprimir
- **Control Principal:** VSPrinter vp
- **Características:**
  - Zoom configurable (4-400%)
  - NavBar (Whole Page, Page Width, Two Pages, Thumbnail)
  - Navegación por páginas
  - Botón "cerrar"
- **Propiedades:** sizefont, sizefontTitle (recibe desde frmprint)

### 6.4 frmTitRep - Título de Reporte (formulario auxiliar)

### 6.5 Clases de Reporte
- **clsReporteCI** - Reporte Con Imágenes (imágenes + texto, grid + tabla Reportes)
- **clsReporteSI** - Reporte Sin Imágenes (tabla datos, tabla reportessi)
- **clsPaginas** - Gestión de paginación (Pagina, Total, Índices, Bookmarks)

---

## 7. TOTALIZACIÓN Y ACTUALIZACIÓN DE DATOS

### 7.1 frmTotalizar - Sumatorios por Campo
**Propósito:** Calcula totales (suma) de campos numéricos/moneda
- **Control:** MSFlexGrid g (2 columnas: Campo | Valor)
- **Propiedades:**
  - AllowUserResizing = flexResizeColumns
  - ScrollBars = flexScrollBarBoth
  - TextMatrix(0,0)="Campo", TextMatrix(0,1)="Valor"
- **Lógica:** Recorre registros visibles, suma campos marcados como totalizables

### 7.2 frmActGrlDat - Actualización General de Datos
**Propósito:** Cambia valor de campo en múltiples registros
- **Controles:**
  - cmbagdd - Selector de campo a actualizar
  - txtagdd(0) - Nuevo valor (TextBox)
  - dtpf(0) - Alternativa para Fecha
  - Check1 - "Solo Seleccionados" (aplica a selección visual)
  - Check2 - "Actualizar valor para imágenes con mismo 'Valor Asignado'..." (propagación)
  - Comaf - Botón búsqueda
  - txtagdd(1), dtpf(1) - Condición secundaria (oculto)
- **Función:** UPDATE masivo en tabla Principal o Variantes

---

## 8. EXPORTACIÓN (frmExp)

**Propósito:** Extrae datos a formato externo (CSV, delimitado, XML)
- **Interfaz:** 2 listboxes
  - lstCE - Campos disponibles (no seleccionados)
  - lstCV - Campos a exportar (seleccionados)
- **Botones:**
  - "Agregar >" (Coms(0)) - Mueve lstCE → lstCV
  - "< Quitar" (Coms(1)) - Mueve lstCV → lstCE
  - "↑" (Coms(2)) - Sube en orden
  - "↓" (Coms(3)) - Baja en orden
- **Lógica:**
  - Excluye campos calculados y campos variantes (si variante=True)
  - Solo texto plano; no exporta imágenes
  - Destino: selector de archivo (clsSeleccion con filtro externo)

---

## 9. HERRAMIENTAS AVANZADAS

### 9.1 frmAlbumsL - Álbumes Ligados
**Propósito:** Gestiona relaciones maestro-detalle entre álbumes
- **Grid:** Lista de álbumes relacionados + llave de ligadura
  - Columnas: Album | Llave | Crear(bandera)
- **Botones:**
  - **Editar** - frmEdligado para modificar liga
  - **Nuevo** - Selecciona álbum a ligar (clsSeleccion)
  - **Eliminar** - Borra liga de grid
  - **Actualiza Liga** - Sincroniza datos desde álbum ligado a registro actual
  - **Actualizar Todas** - Sincroniza desde todas las ligas (con progress bar)
- **Función frmEdligado:**
  - Combo para seleccionar campo clave (debe existir en álbum destino)
  - Check: "Dar de alta Imagen si no existe..." (crea registro si no coincide llave)
- **Almacenamiento:** Tabla ligados (álbum, campo llave, flags)
- **Lógica:** LEFT JOIN conceptual, actualiza campos desde registro coincidente

### 9.2 frmstligas - Status de Ligadura (Proceso)
**Propósito:** Indicador visual de progreso durante sincronización
- **Grid:** Tabla con estado
- **Botón:** "Cerrar" (deshabilitado hasta terminar)

### 9.3 frmCopAlbum / Dialog (frmCopiar) - Copiar Álbum
**Propósito:** Duplica álbum completo o solo estructura
- **Diálogo (frmCopiar.Dialog):**
  - Check: "Copiar Solo Estructura"
  - TextBox: "Guardar Como:" + botón "Examinar"
- **Funciones:**
  - Si solo estructura: copia tabla de propiedades y esquema de tablas, vacío
  - Si con datos: copia registros completos (Principal, Variantes, Multidatos)
- **Alternativa:** frm3Botones (3 botones) - "Solo Estructura", otros modos

### 9.4 frmNuevo → frmnewp (Modificar Álbum)
**Propósito:** Edita estructura de álbum existente
- **Modo:** ModificaAlbum = True
- **Cambios permitidos:**
  - Renombrar campos (marca como "_nuevo_")
  - Agregar campos nuevos
  - Cambiar longitud/decimales
  - Cambiar totalizable
  - NO eliminar ni reordenar campos existentes
- **Almacenamiento:** 
  - Si NUEVO: crea .mdb nuevo
  - Si SOBREESCRIBIR: guarda en temp y reemplaza

### 9.5 frmDALV - Datos A La Vista
**Propósito:** Configura campos visibles en vista principal + modificables
- **Interfaz:**
  - lstCE - Campos no visibles
  - lstCV - Campos visibles (con checkbox para modificable)
  - Botones: Agregar, Quitar, Subir, Bajar, Todos, Todos<<
- **Restricciones:**
  - Variantes: solo 1 campo visible, no modificable
  - Calculados: nunca modificables
  - Multidatos: no se muestran en vista
- **Resultado:** Actualiza propiedades visible/modificable/OrdenVisible en clsImgCampos

---

## 10. VISOR DE IMÁGENES

**frmDocument / MuestraImagen:**
- Abre imagen a 100% en nueva ventana (no hay frm específico, usa Image en frmDocument)
- Visualización a escala completa
- Alternativa: visor.exe externo (mencionado pero no implementado aquí)

---

## 11. ESTRUCTURA DE DATOS (db.bas + Module1.bas)

### 11.1 Tipos de Campo (Enum TipoCar)
```
TC_TEXTO = 0
TC_NUMERICO = 1
TC_MONEDA = 2
TC_FECHA = 3
TC_CALCU = 4
TC_MULTID = 5
```

### 11.2 Tablas Base en Access (.mdb)
- **Principal** - Registro principal (todos los campos definidos + _imagen_, _id_, _auxiliar_, _variantes_)
- **Variantes** - Registros secundarios (_imagen_, _id_, _idprincipal_)
- **Multidatos** - Valores múltiples de campos multidato (Id, Principal, Campo_n, Valor)
- **Propiedades** - Metadatos de campos (Nombre, Tipo, longitud, decimales, totalizable, sInfo, TipoSal, Enprincipal, Modificable, Visible, OrdenVisible)
- **Categorias** - Palabras predefinidas por campo (Campo_n, Principal, Valor, Default)
- **Reportes** - Configuración de reportes con imagen (clsReporteCI)
- **ReporteSI** - Configuración de reportes sin imagen (clsReporteSI)
- **FiltrosAv** - Filtros guardados (Nombre, OpR, Campo, OpC, Valor)
- **Grupos** - Agrupaciones de registros (nombre, descripción, status)
- **ligados** - Álbumes relacionados (álbum, campo llave, flags)

### 11.3 Campos Reservados
- `_imagen_` - Ruta archivo imagen (TEXT 255)
- `_id_` - Identificador único (LONG, GenUniqueID())
- `_auxiliar_` - Bandera auxiliar (BOOLEAN)
- `_variantes_` - Indica existencia de variantes (BOOLEAN, solo en Principal)
- `_idprincipal_` - Referencia a registro principal (LONG, solo en Variantes)

### 11.4 Constantes Importantes (Module1.bas)
- `MAX_TEXT = 255` - Longitud máxima de texto
- `MIC_VAR = "_variante_"` - Nombre nodo variante (XML)
- `MIC_IMG = "_imagen_"` - Nombre nodo imagen (XML)
- `MIC_AUX = "_auxiliar_"` - Nombre nodo auxiliar
- `INI_CON` - Conexión: "DRIVER=Microsoft Access Driver (*.mdb);DBQ="
- `SEP_V = "|"` - Separador vertical (en exportación)
- `CLRSEL_ON, CLRSEL_OFF` - Colores selección (rojo/gris)

---

## 12. PLANTILLAS Y ESTRUCTURA

### 12.1 Plantillas (.xms - XML)
- **Formato:** XML con documento DOMDocument
- **Estructura:**
  ```xml
  <Album>
    <Registro>
      <Campo1>...</Campo1>
      <_imagen_/>
      <_variante_>
        <Registro>
          <CampoVar1/>
        </Registro>
      </_variante_>
    </Registro>
  </Album>
  ```
- **Almacenamiento:** Directorio DirPlantillas (default: {AppPath}\plantillas)
- **Directorios:**
  - DirAlbums - Álbumes (.mdb), default {AppPath}\albums
  - DirImages - Imágenes origen, default {AppPath}\imagenes
  - DirEmpImp - Paquetes empacados, default {AppPath}\EmpImp

### 12.2 Empaquetado (clsGrupo)
- **Comando:** EmpacarAlbum (Herramientas)
- **Proceso:** Comprime .mdb + todas las imágenes en archivo único
- **Desempacar:** Frm3Botones → DesempacarAlbum (reconstruye estructura)

---

## 13. CLASES Y COMPONENTES

### 13.1 Clases de Datos
- **clsImagenMic** - Propiedades de un campo (nombre, tipo, longitud, decimales, totalizable, visible, modificable, fórmula, etc.)
- **clsImgCampos** - Colección de clsImagenMic (acceso por índice o nombre)
- **clsSeleccion** - Diálogo de archivo reutilizable
- **Campos** - Propiedades de estructura de campo

### 13.2 Clases de Reporte
- **clsReporteCI** - Configuración reporte con imágenes
- **clsReporteSI** - Configuración reporte sin imágenes
- **clsPaginas** - Gestión de paginación

### 13.3 Clases Auxiliares
- **clsGrupo** - Agrupación de registros (jerarquía)
- **clsArchivoSel** - Selección múltiple de archivos
- **clsPregunta** - Pregunta con opciones (ventana modal)
- **clsdriver** - Generador de claves (protección anticopia)
- **SysInfo** - Información del sistema (para registro)

---

## 14. MENÚ CONTEXTO Y FUNCIONES ESPECIALES

### 14.1 Contexto sobre Imagen
- Nueva, Editar, Eliminar, Ocultar, Campos a la Vista, Ver 100%

### 14.2 Contexto sobre Multidato
- Borrar, Editar (valores predefinidos)

### 14.3 Accesos Rápidos (Toolbar)
- Nueva imagen, Abrir, Escalas (8x, 4x, 2x), Navegación, Buscar, Ordenar, ActGral, Totalizar

---

## 15. BARRA DE ESTADO (StatusBar - 10 paneles)

1. **Status** (autosize) - Mensaje general
2. **OcultarPanel** - Contraer/expandir panel lateral
3. **Rango** - "0-0/0" (imágenes en pantalla / total)
4. **Primera** (imagen icon) - Botón navegación
5. **Anterior** (imagen icon) - Botón navegación
6. **Siguiente** (imagen icon) - Botón navegación
7. **Última** (imagen icon) - Botón navegación
8. **Filtrado** - Indicador estado filtro (ancho 2469)
9. **Ordenación** - Indicador orden actual (texto "SIN" por default)
10. **Refresco** - Indicador refresco en curso

---

## 16. FUNCIONALIDADES MENORES Y DETALLES

### 16.1 Contadores y Estado
- Panel estado muestra: "Estado", rango visible, total registros
- barra de navegación con página actual/total
- Indicador de filtrado activo en status
- Indicador de orden actual

### 16.2 Zoom y Escala
- 3 escalas de thumbnails: 8, 4, 2 por línea (configurable en toolbar)
- Botones alternativos en menú (aunque ocultos en frmMain)

### 16.3 Selección Múltiple
- Click en Marco para seleccionar imagen
- Shift+Click para rangos
- Ctrl+Click para seleccionar no adyacentes
- Menú: Selecciona Todos, Invierte Selección
- Color de selección: CLRSEL_ON (rojo) vs CLRSEL_OFF (gris)

### 16.4 Protección Anticopia
- Comando "Desinstalar" genera código de desinstalación
- clsdriver.GeneraSolucion(code1) → CD
- Vincula a máquina específica (sin código de desinstalación previo, app inservible)

### 16.5 Asociación de Extensión
- Comando "Asociar el mic a .mdb" (creaASOC)
- Vincula .mdb a programa MIC en registro Windows
- Doble clic en .mdb abre directamente en MIC

### 16.6 Backup y Respaldo
- Comando "&Usar Respaldo" (deshabilitado/no implementado)
- Prefijo backup: B_PRE = "~$", extensión: B_EXT = ".bak"

### 16.7 Cálculos de Campos Computados
- Fórmulas que referencia otros campos
- Recalculación manual: comando "Act. Calculados"
- Validación de referencias circulares en frmCaptForm

---

## 17. DIRECTORIOS Y CONFIGURACIÓN

- **Directorio Plantillas:** DirPlantillas (registry: Settings/Plantillas)
- **Directorio Álbumes:** DirAlbums (registry: Settings/Albums)
- **Directorio Imágenes:** DirImages (registry: Settings/Imagenes)
- **Directorio Empacados:** DirEmpImp (registry: Settings/EmpacadosEImportados)
- **Persistencia:** GetSetting/SaveSetting en "mic" (App.title)
- **Posición ventana:** MainLeft, MainTop, MainWidth, MainHeight

---

## 18. FLUJOS PRINCIPALES

### 18.1 Crear Álbum Nuevo
1. Menú Archivo → Nuevo Album (mnuFileNew_Click)
2. Llama NuevoSetPlantilla → abre frmNuevo
3. Usuario elige plantilla o "Nuevo Album"
4. Si plantilla: carga .xms
5. Abre frmnewp para editar campos (si nuevo)
6. Presiona "Guardar Album" → CreaAlbum (crea .mdb, tablas, estructura)
7. LoadNewDoc abre frmDocument (ventana MDI)

### 18.2 Abrir Álbum Existente
1. Menú Archivo → Abrir (mnuFileOpen_Click)
2. clsSeleccion.seleccionar → elige .mdb
3. AbrirDocAlbum → abre Base de datos
4. LoadNewDoc abre frmDocument, carga estructura desde tabla propiedades

### 18.3 Capturar Datos
1. Doble clic en thumbnail o menú Imagen → Nueva/Editar
2. CapturaCampos → abre frmCaptura (modal)
3. Usuario selecciona imagen, llena campos
4. Aceptar → guarda en tabla Principal/Variantes + imagen en DirImages

### 18.4 Buscar Registro
1. Toolbar/Menú → Buscar
2. frmBuscar modal
3. Usuario ingresa criterio + opciones
4. localiza (ipos=0: desde inicio, ipos>0: desde siguiente)
5. BuscaEnDatosDeImagen busca por tipo de campo
6. Resultado: selecciona imagen, navega a página

### 18.5 Filtrar por Criterios Avanzados
1. Menú Herramientas → Ordenar / o pestaña Filtros en frmDocument
2. frmFA → define AND/OR de criterios (hasta 3)
3. Aceptar → recalcula Paginas, muestra solo registros coincidentes
4. Status bar indicador de filtrado

### 18.6 Imprimir Reporte
1. Menú Archivo → Imprimir (frmprint) o ReporteSinImg (frmprint2)
2. Selecciona reporte guardado o configura nuevo
3. Elige campos, orden, líneas por campo
4. Vista Preliminar (frmPreliminar) con zoom
5. Envía a impresora o archivo PDF

---

## 19. RESUMEN DE CARACTERÍSTICAS VISIBLES AL USUARIO

| Funcionalidad | Formulario/Menú | Detalles |
|---|---|---|
| Crear álbum | Archivo → Nuevo Album | Plantilla o nuevo desde cero |
| Abrir álbum | Archivo → Abrir | Selector de .mdb |
| Guardar | Archivo → Guardar | Persiste cambios |
| Copiar álbum | Archivo → Copiar Album | Estructura o con datos |
| Nueva imagen | Archivo → Nueva Imagen | Selecciona archivo, asigna campo imagen |
| Editar imagen | Contexto derecho | frmCaptura, valida multidatos |
| Eliminar imagen | Contexto derecho | Requiere selección |
| Ocultar imagen | Contexto derecho | Sin borrar |
| Captura multidatos | frmCaptura + ListBox | Agregar/editar/borrar valores |
| Autofill / Búsqueda | frmAutofill | Listado de valores predefinidos |
| Buscar registro | Toolbar/Menú | frmBuscar, por texto/número/fecha |
| Filtro avanzado | frmFA | AND/OR, hasta 3 criterios |
| Ordenar | frmOrdenar | 3 niveles + opcional agrupar |
| Agrupar registros | frmOrdenar (modo grupo) | Por campo primario |
| Totalizar | frmTotalizar | Sumatorios de campos numéricos |
| Actualización masiva | frmActGrlDat | Cambia valor en múltiples registros |
| Escalas thumb | Toolbar | 8x, 4x, 2x imágenes |
| Navegación página | Toolbar / StatusBar | Primera, anterior, siguiente, última |
| Imprimir catálogo | frmprint | Con imágenes + datos |
| Reporte tabla | frmprint2 | Datos sin imágenes, agrupable |
| Exportar datos | frmExp | Campos seleccionables, formato texto |
| Campos visibles | frmDALV | Configura visible + modificable |
| Campos editar | frmEdCmps/frmauxcmp | Define tipos, longitud, fórmulas |
| Variantes | frmnewp | Sub-registros por imagen |
| Plantillas | Archivo → Nuevo Album | .xms reutilizable |
| Álbumes ligados | Herramientas → Albums Ligados | Relaciones maestro-detalle |
| Empacar/desempacar | Herramientas → Empacar/Desempacar | Compress .mdb + imágenes |
| Protección | Herramientas → Desinstalar | Código de desinstalación |
| Asociar .mdb | Herramientas → Asociar | Registro Windows |
| Acerca de | Ayuda → Acerca de | frmAbout, versión y créditos |

---

## LISTA COMPLETA DE 32 FORMULARIOS VB6

1. **frmMain** - MDI Principal, menús y barra de herramientas
2. **frmDocument** - Ventana MDI secundaria para edición de álbum (grid de imágenes)
3. **frmNuevo** - Selección de plantilla para álbum nuevo
4. **frmnewp** - Editor de estructura de álbum/plantilla (grid de campos)
5. **frmEdCmps** - Editor alternativo de estructura (radio buttons)
6. **frmauxcmp** - Editor auxiliar de campo individual
7. **frmCaptForm** - Editor de fórmula para campos calculados
8. **frmCaptura** - Captura/edición modal de registro con controles dinámicos
9. **frmAutofill** - Lista predefinida de valores (ListView + botones CRUD)
10. **frmBuscar** - Búsqueda por texto/valor con opciones case-sensitive
11. **frmFA** - Filtro avanzado multi-criterio (AND/OR)
12. **frmAbreFA** - Selector de filtros guardados
13. **frmFiltros** - Dialog básico de filtros (stub)
14. **frmOrdenar** - Ordenamiento 3-niveles + modo agrupar
15. **frmprint** - Imprimir catálogo con imágenes (clsReporteCI)
16. **frmprint2** - Imprimir tabla sin imágenes (clsReporteSI)
17. **frmPreliminar** - Vista preliminar con zoom y navegación
18. **frmTitRep** - Título de reporte (auxiliar)
19. **frmTotalizar** - Sumatorios de campos numéricos (MSFlexGrid)
20. **frmActGrlDat** - Actualización general masiva de datos
21. **frmExp** - Exportación selectiva de campos
22. **frmDALV** - Configuración de campos visibles/modificables
23. **frmAlbumsL** - Gestión de álbumes ligados (maestro-detalle)
24. **frmEdligado** - Configuración de liga individual
25. **frmstligas** - Indicador de progreso durante sincronización
26. **frmCopAlbum** - Diálogo para copiar álbum (estructura/datos)
27. **frmCopiar** - Alternativa: Dialog para copiar con ruta
28. **frm3Botones** - Multi-propósito: 3 opciones de botones (Desempacar, etc.)
29. **frmAbout** - Acerca de... (versión, créditos)
30. **frmSplash** - Pantalla de inicio (mencionada pero no analizada en detalle)
31. **frmkey** - Diálogo de registro/protección anticopia
32. **clsSeleccion** (no es formulario pero es componente reutilizable)

---

## MÓDULOS VB6

- **Module1.bas** - Enums (TipoCar, clrsel, GR_STATUS), constantes globales, función creaASOC, Aplicafmto, CapturaCampos
- **Module2.bas** - Funciones auxiliares (sin detalles específicos en análisis)
- **Module3.bas** - Funciones auxiliares
- **Module4.bas** - Funciones auxiliares
- **Module5.bas** - Funciones auxiliares
- **Module6.bas** - Funciones auxiliares
- **db.bas** - Funciones de BD: CreaTPrinVar, CreaTMult, CreaTCategorias, cCargaCateg, SalvaFA, SalvaPropCampo, GetMultidatos, GuardaMultid

---

**FIN DEL INVENTARIO FUNCIONAL COMPLETO**

Documento de especificación funcional para migración de MIC (VB6 + Access) a Tauri + Svelte.
