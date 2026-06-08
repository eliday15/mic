/**
 * Cadenas de interfaz en español (es-MX). Única fuente de texto visible.
 *
 * Estructurado por área para que las vistas importen `t` y accedan a la rama
 * que necesiten. Sin pluralización compleja: el dominio no la requiere.
 */

export const t = {
  app: {
    titulo: "MIC",
    subtitulo: "Catálogo de inventario",
    cargando: "Cargando…",
    sinAlbum: "No hay ningún álbum abierto",
    sinAlbumDesc: "Crea un álbum nuevo o abre uno existente para comenzar.",
  },

  // ----------------------------------------------------------------------
  // Barra de menú
  // ----------------------------------------------------------------------
  menu: {
    archivo: "Archivo",
    editar: "Editar",
    ver: "Ver",
    herramientas: "Herramientas",
    ayuda: "Ayuda",
  },

  archivo: {
    nuevoAlbum: "Nuevo Álbum",
    abrir: "Abrir…",
    recientes: "Recientes",
    sinRecientes: "Sin álbumes recientes",
    cerrarAlbum: "Cerrar Álbum",
    compactar: "Compactar Álbum",
    copiarAlbum: "Copiar Álbum…",
    empacar: "Empacar Álbum…",
    desempacar: "Desempacar Álbum…",
    exportar: "Exportar…",
    imprimir: "Imprimir…",
    importar: "Importar desde Access…",
    salir: "Salir",
  },

  editar: {
    nuevaImagen: "Nueva Imagen",
    nuevaVariante: "Nueva Variante",
    editarRegistro: "Editar Registro",
    duplicar: "Duplicar",
    ocultar: "Ocultar",
    mostrar: "Mostrar (quitar oculto)",
    eliminar: "Eliminar",
    seleccionarTodo: "Seleccionar Todo",
    deseleccionar: "Deseleccionar Todo",
    invertirSeleccion: "Invertir Selección",
    deshacer: "Deshacer",
    rehacer: "Rehacer",
    copiar: "Copiar",
    pegar: "Pegar",
  },

  ver: {
    grilla: "Grilla",
    tabla: "Tabla",
    aumentar: "Aumentar Miniaturas",
    disminuir: "Reducir Miniaturas",
    panelFiltros: "Panel de Filtros",
    panelGrupos: "Panel de Grupos",
    inspector: "Inspector",
    camposVista: "Campos a la Vista…",
    mostrarOcultos: "Mostrar Ocultos",
    visor: "Ver Imagen al 100%",
    tema: "Tema",
    temaOscuro: "Oscuro",
    temaClaro: "Claro",
  },

  herramientas: {
    buscar: "Buscar",
    ordenar: "Ordenar",
    filtros: "Filtros",
    grupos: "Grupos",
    totalizar: "Totalizar",
    actMasiva: "Actualización Masiva de Datos…",
    imagenesCarpeta: "Añadir Imágenes de Carpeta…",
    recalcular: "Recalcular Campos Calculados",
    ligados: "Álbumes Ligados…",
    campos: "Campos del Álbum",
    categorias: "Categorías",
    formulas: "Editor de Fórmulas",
  },

  ayuda: {
    acercaDe: "Acerca de MIC",
    documentacion: "Documentación",
  },

  // ----------------------------------------------------------------------
  // Acciones comunes
  // ----------------------------------------------------------------------
  accion: {
    aceptar: "Aceptar",
    cancelar: "Cancelar",
    guardar: "Guardar",
    aplicar: "Aplicar",
    cerrar: "Cerrar",
    eliminar: "Eliminar",
    quitar: "Quitar",
    agregar: "Agregar",
    nuevo: "Nuevo",
    editar: "Editar",
    abrir: "Abrir",
    examinar: "Examinar…",
    limpiar: "Limpiar",
    restablecer: "Restablecer",
    si: "Sí",
    no: "No",
    continuar: "Continuar",
    volver: "Volver",
    siguiente: "Siguiente",
  },

  // ----------------------------------------------------------------------
  // Búsqueda / orden / filtros / grupos
  // ----------------------------------------------------------------------
  busqueda: {
    placeholder: "Buscar…",
    limpiar: "Limpiar búsqueda",
    sinResultados: "No se encontraron registros",
    sinResultadosDesc: "Ajusta la búsqueda o los filtros activos.",
  },

  orden: {
    titulo: "Ordenar registros",
    por: "Ordenar por",
    luego: "Luego por",
    ascendente: "Ascendente",
    descendente: "Descendente",
    ninguno: "(sin orden)",
    maximoNiveles: "Máximo tres niveles de orden",
  },

  filtros: {
    titulo: "Filtros avanzados",
    rapido: "Filtro rápido",
    condicion: "Condición",
    agregarCondicion: "Agregar condición",
    quitarCondicion: "Quitar condición",
    campo: "Campo",
    operador: "Operador",
    valor: "Valor",
    conector: "Conector",
    guardarComo: "Guardar filtro como…",
    guardados: "Filtros guardados",
    nombreFiltro: "Nombre del filtro",
    sinFiltros: "No hay filtros guardados",
    activo: "Filtro activo",
    limpiarTodo: "Quitar todos los filtros",
    op: {
      igual: "igual a",
      distinto: "distinto de",
      mayor: "mayor que",
      menor: "menor que",
      mayor_igual: "mayor o igual que",
      menor_igual: "menor o igual que",
      contiene: "contiene",
      empieza: "empieza con",
    },
    rel: {
      y: "Y",
      o: "O",
    },
  },

  grupos: {
    titulo: "Grupos",
    nuevo: "Nuevo grupo",
    nombre: "Nombre del grupo",
    por: "Agrupar por",
    luego1: "Luego por",
    luego2: "Y luego por",
    todos: "Todos",
    sinGrupos: "No hay grupos definidos",
    eliminarGrupo: "Eliminar grupo",
    conteo: "registros",
  },

  totalizar: {
    titulo: "Totalizar y estadísticas",
    campo: "Campo",
    total: "Total",
    suma: "Suma",
    promedio: "Media",
    mediana: "Mediana",
    moda: "Moda",
    cuenta: "Cuenta",
    minimo: "Mín",
    maximo: "Máx",
    sinTotalizables: "No hay campos numéricos",
    sinTotalizablesDesc:
      "Agrega campos de número, moneda o calculados para analizarlos aquí.",
    registros: "Registros en el conjunto",
    elegirCampos: "Campos a analizar",
    ordenarGrilla: "Ordenar la grilla por este campo",
    clicCopia: "Clic en un valor lo copia · clic en una columna ordena la tabla",
    veces: "veces",
    nota: "Los cálculos respetan el filtro, grupo y búsqueda activos.",
  },

  actMasiva: {
    titulo: "Actualización masiva de datos",
    campo: "Campo a actualizar",
    nuevoValor: "Nuevo valor",
    alcance: "Aplicar a",
    seleccionados: "Solo los registros seleccionados",
    filtrados: "Todos los registros del filtro actual",
    todos: "Todos los registros del álbum",
    aplicar: "Actualizar",
    confirmacion: "Esta acción modificará varios registros a la vez.",
    resultado: "registros actualizados",
  },

  copiarAlbum: {
    titulo: "Copiar álbum",
    destino: "Guardar copia como",
    soloEstructura: "Copiar solo la estructura (sin registros ni imágenes)",
    copiando: "Copiando álbum…",
    resultado: "Álbum copiado",
    imagenes: "imágenes copiadas",
  },

  visor: {
    ajustar: "Ajustar a la ventana",
    tamanoReal: "Tamaño real (100 %)",
    acercar: "Acercar",
    alejar: "Alejar",
    anterior: "Imagen anterior",
    siguiente: "Imagen siguiente",
    sinImagen: "Este registro no tiene imagen",
  },

  exportar: {
    titulo: "Exportar registros",
    disponibles: "Campos disponibles",
    incluidos: "Campos a exportar",
    agregar: "Agregar →",
    quitar: "← Quitar",
    subir: "Subir",
    bajar: "Bajar",
    formato: "Formato",
    csv: "CSV (texto separado por comas)",
    xlsx: "Excel (.xlsx)",
    csvMic: "CSV para MIC clásico (2007)",
    destino: "Guardar como",
    exportando: "Exportando…",
    resultado: "registros exportados",
    nota: "Se exporta el conjunto filtrado actual, en el orden activo.",
    notaMic:
      "Para el MIC clásico: el primer campo de la lista es la llave. " +
      "Impórtalo en el MIC 2007 con Archivo → Importar… " +
      "(las comas dentro de valores se sustituyen por espacios).",
    sinCampos: "Agrega al menos un campo para exportar",
  },

  reportes: {
    titulo: "Imprimir",
    tituloSin: "Reporte sin imágenes",
    reporte: "Reporte",
    nuevoReporte: "(nuevo reporte)",
    nombre: "Nombre del reporte",
    tipo: "Tipo de reporte",
    conImagenes: "Catálogo con imágenes",
    sinImagenes: "Tabla de datos (sin imágenes)",
    tituloDoc: "Título del documento",
    imagenesPorLinea: "Imágenes por línea",
    camposIncluidos: "Campos a imprimir",
    orientacion: "Orientación",
    vertical: "Vertical",
    horizontal: "Horizontal",
    tamanoPapel: "Tamaño de papel",
    carta: "Carta",
    oficio: "Oficio",
    a4: "A4",
    ponFecha: "Incluir fecha",
    ponPagina: "Incluir número de página",
    ponTotales: "Incluir totales",
    agrupacion: "Agrupar por",
    sinAgrupacion: "(sin agrupación)",
    guardarReporte: "Guardar reporte",
    eliminarReporte: "Eliminar reporte",
    vistaPrevia: "Vista previa",
    imprimir: "Imprimir",
    generando: "Generando vista previa…",
    pagina: "Página",
    de: "de",
    fecha: "Fecha",
    total: "Total",
    registros: "registros",
  },

  ligados: {
    titulo: "Álbumes ligados",
    descripcion:
      "Sincroniza campos desde otro álbum usando un campo llave en común.",
    album: "Álbum ligado",
    llave: "Campo llave",
    crearSiNoExiste: "Dar de alta el registro si la llave no existe",
    nuevo: "Nueva liga…",
    editarLiga: "Editar liga",
    eliminarLiga: "Eliminar liga",
    actualizar: "Actualizar liga",
    actualizarTodas: "Actualizar todas",
    sinLigas: "No hay álbumes ligados",
    elegirAlbum: "Elegir álbum…",
    progreso: "Sincronizando…",
    resultado: {
      actualizados: "actualizados",
      creados: "creados",
      sinCoincidencia: "sin coincidencia",
    },
  },

  importacion: {
    accionMenu: "Importar registros (CSV/Excel)…",
    titulo: "Importar registros",
    // Fase elegir
    archivo: "Archivo de origen (CSV o Excel)",
    examinar: "Examinar…",
    formatos: "Se aceptan .csv (UTF-8 o Windows-1252) y .xlsx (primera hoja).",
    // Fase configurar
    columnasReconocidas: "Columnas reconocidas",
    columnasNoReconocidas: "Columnas ignoradas (no coinciden con ningún campo)",
    encoding: "Codificación detectada",
    totalFilas: "Filas en el archivo",
    campoLlave: "Campo llave (para casar registros)",
    campoLlaveNota:
      "El campo llave nunca se modifica y distingue mayúsculas y acentos.",
    politica: "Al coincidir la llave",
    sustituir: "Sustituir: sobrescribir con los valores del archivo",
    mantener: "Mantener: conservar los valores actuales",
    rellenarVacios: "Rellenar vacíos: solo escribir campos hoy vacíos",
    crearFaltantes: "Crear registros para llaves no encontradas",
    verResumen: "Ver resumen",
    // Progreso
    analizando: "Analizando…",
    aplicando: "Importando…",
    // Resumen (dry-run)
    resumenTitulo: "Resumen previo",
    seActualizaran: "se actualizarán",
    seCrearan: "se crearán",
    sinCambio: "sin cambio",
    avisos: "Avisos",
    errores: "Errores",
    sinAvisos: "Sin avisos",
    aplicar: "Aplicar importación",
    volver: "Volver",
    // Resultado
    resultadoTitulo: "Importación completada",
    actualizados: "actualizados",
    creados: "creados",
    sinCambioRes: "sin cambio",
  },

  // ----------------------------------------------------------------------
  // Registros / editor
  // ----------------------------------------------------------------------
  registro: {
    nuevo: "Nuevo registro",
    editar: "Editar registro",
    imagen: "Imagen",
    elegirImagen: "Elegir imagen…",
    quitarImagen: "Quitar imagen",
    sinImagen: "Sin imagen",
    variantes: "Variantes",
    nuevaVariante: "Nueva variante",
    editarVariante: "Editar variante",
    volverPrincipal: "Volver al principal",
    sinVariantes: "Sin variantes",
    calculadoSoloLectura: "Campo calculado (solo lectura)",
    multidato: "Etiquetas",
    agregarValor: "Agregar etiqueta",
    seleccion: "seleccionados",
    seleccionParcial: (n: number, total: number): string =>
      `Seleccionados ${n} de ${total} (solo los registros ya cargados)`,
    sinSeleccionInspector: "Selecciona un registro para editarlo aquí",
    edicionLote: "Edición en lote",
    edicionLoteDesc: "Los cambios se aplicarán a todos los registros seleccionados.",
  },

  campos: {
    titulo: "Campos del álbum",
    nuevo: "Nuevo campo",
    nombre: "Nombre",
    tipo: "Tipo",
    decimales: "Decimales",
    totalizable: "Totalizable",
    visible: "Visible",
    modificable: "Modificable",
    formula: "Fórmula",
    tabla: "Tabla",
    reordenar: "Arrastra para reordenar",
    tipos: {
      texto: "Texto",
      numerico: "Número",
      moneda: "Moneda ($)",
      fecha: "Fecha",
      calculado: "Calculado",
      multidato: "Etiquetas",
    },
    formato: "Formato",
    formatoSimple: "Número simple",
    formatoPorcentaje: "Porcentaje (%)",
    tabla_: {
      principal: "Principal",
      variantes: "Variantes",
    },
  },

  camposVista: {
    titulo: "Campos a la vista",
    descripcion:
      "Elige qué campos se muestran y cuáles se pueden editar. Arrastra para cambiar el orden de las columnas y etiquetas.",
    visible: "Visible",
    modificable: "Modificable",
    siempreLectura: "Los campos calculados son siempre de solo lectura.",
  },

  categorias: {
    titulo: "Categorías",
    valor: "Valor",
    porDefecto: "Por defecto",
    agregar: "Agregar categoría",
    sinCategorias: "No hay categorías definidas",
  },

  formulas: {
    titulo: "Editor de fórmulas",
    expresion: "Expresión",
    probar: "Probar",
    vistaPrevia: "Vista previa",
    error: "Error en la fórmula",
    campos: "Campos",
    operadores: "Operadores",
    numero: "Número",
    insertarNumero: "Insertar número",
    quitarUltimo: "Quitar lo último",
    limpiarFormula: "Limpiar fórmula",
    ayuda:
      "Arma la fórmula con clics: un campo, un operador, otro campo o número. También puedes escribirla (espacios del nombre → guion bajo).",
  },

  // ----------------------------------------------------------------------
  // Álbum nuevo / abrir
  // ----------------------------------------------------------------------
  nuevoAlbum: {
    titulo: "Crear álbum nuevo",
    nombre: "Nombre del álbum",
    ubicacion: "Ubicación",
    campos: "Campos iniciales",
    crear: "Crear álbum",
    yaExiste:
      "Ya existía un álbum en esa ruta; te sugerí un nombre nuevo. Vuelve a pulsar Crear álbum.",
    formulaPlaceholder: "Precio * Cantidad",
    formulaNota: "Podrás ajustar la fórmula después en Herramientas → Campos del Álbum.",
  },

  // ----------------------------------------------------------------------
  // Migración
  // ----------------------------------------------------------------------
  migracion: {
    titulo: "Importar desde Access",
    archivoMdb: "Archivo .mdb de origen",
    destino: "Álbum de destino",
    inspeccionar: "Inspeccionar",
    ejecutar: "Ejecutar migración",
    verificandoHerramientas: "Verificando herramientas de migración…",
    faltanHerramientas: "No se encontró mdb-tools en el sistema",
    faltanHerramientasDesc:
      "Instala mdb-tools para poder importar archivos de Access.",
    inspeccion: {
      tablas: "Tablas",
      campos: "Campos",
      totalEstimado: "Registros estimados",
      tieneVariantes: "Incluye variantes",
    },
    progreso: {
      fase: "Fase",
      preparando: "Preparando…",
      registros: "Migrando registros…",
      imagenes: "Copiando imágenes…",
      finalizando: "Finalizando…",
    },
    reporte: {
      titulo: "Migración completada",
      filasPrincipal: "Registros principales",
      filasVariantes: "Variantes",
      filasMultidatos: "Valores múltiples",
      imagenesEncontradas: "Imágenes copiadas",
      imagenesFaltantes: "Imágenes faltantes",
      advertencias: "Advertencias",
      sinAdvertencias: "Sin advertencias",
    },
  },

  // ----------------------------------------------------------------------
  // Confirmaciones
  // ----------------------------------------------------------------------
  confirmar: {
    eliminarRegistros: "¿Eliminar los registros seleccionados?",
    eliminarRegistrosDesc: "Esta acción no se puede deshacer.",
    eliminarCampo: "¿Eliminar este campo?",
    eliminarCampoDesc: "Se perderán todos los datos asociados a este campo.",
    eliminarGrupo: "¿Eliminar este grupo?",
    eliminarFiltro: "¿Eliminar este filtro guardado?",
    eliminarPlantilla: "¿Eliminar esta plantilla?",
    cerrarAlbum: "¿Cerrar el álbum?",
    cambiosSinGuardar: "Hay cambios sin guardar",
    cambiosSinGuardarDesc: "¿Deseas descartar los cambios?",
    descartar: "Descartar",
  },

  // ----------------------------------------------------------------------
  // Mensajes de estado y error
  // ----------------------------------------------------------------------
  mensaje: {
    guardado: "Cambios guardados",
    eliminado: "Eliminado correctamente",
    creado: "Creado correctamente",
    copiado: "Copiado al portapapeles",
    albumCreado: "Álbum creado",
    albumAbierto: "Álbum abierto",
    migracionOk: "Migración completada con éxito",
    sinSeleccion: "No hay registros seleccionados",
    operacionCancelada: "Operación cancelada",
    ocultados: "Registros ocultados",
    mostrados: "Registros visibles de nuevo",
    recalculados: "campos recalculados",
    actualizados: "registros actualizados",
    empacado: "Álbum empacado",
    desempacado: "Álbum desempacado",
  },

  error: {
    generico: "Ocurrió un error",
    cargarAlbum: "No se pudo abrir el álbum",
    crearAlbum: "No se pudo crear el álbum",
    cargarRegistros: "No se pudieron cargar los registros",
    guardarRegistro: "No se pudo guardar el registro",
    eliminarRegistro: "No se pudieron eliminar los registros",
    cargarImagen: "No se pudo cargar la imagen",
    formula: "Error al evaluar la fórmula",
    migracion: "Error durante la migración",
    importacion: "Error durante la importación",
    valorInvalido: "Valor inválido",
    campoRequerido: "Este campo es obligatorio",
    fechaInvalida: "Use el formato AAAA-MM-DD",
    numeroInvalido: "Debe ser un número",
  },

  // ----------------------------------------------------------------------
  // Estado vacío genérico / accesibilidad
  // ----------------------------------------------------------------------
  vacio: {
    titulo: "Nada que mostrar",
    descripcion: "No hay elementos para esta vista.",
    albumTitulo: "Álbum vacío",
    albumDesc:
      "Arrastra imágenes aquí, usa Nueva Imagen (⌘I) o Herramientas → Añadir Imágenes de Carpeta.",
  },

  plantillas: {
    titulo: "Plantilla",
    desdeCero: "(desde cero)",
    guardarComo: "Guardar como plantilla…",
    eliminar: "Eliminar plantilla",
    nombre: "Nombre de la plantilla",
    guardada: "Plantilla guardada",
    eliminada: "Plantilla eliminada",
  },

  a11y: {
    cerrarDialogo: "Cerrar diálogo",
    menuContextual: "Menú contextual",
    expandir: "Expandir",
    contraer: "Contraer",
    seleccionar: "Seleccionar",
    cargando: "Cargando",
  },
} as const;

/** Tipo de la tabla de cadenas (para tipar accesos en componentes). */
export type Textos = typeof t;
