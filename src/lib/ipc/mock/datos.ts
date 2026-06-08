/**
 * Datos de demostración para el modo navegador (mock IPC).
 *
 * Genera en memoria un álbum realista ("Catálogo Demo") con campos de todos
 * los tipos, registros con variantes, multidatos, categorías, grupos y filtros
 * guardados. La generación es determinista (LCG con semilla fija) para que las
 * revisiones automatizadas sean reproducibles.
 *
 * SOLO se usa en desarrollo cuando la app corre en un navegador sin Tauri.
 */

import type {
  CampoDef,
  CategoriaVal,
  CondicionFiltro,
  Grupo,
  Valores,
} from "$lib/domain/types";

/** Registro en memoria del mock (forma interna, no del contrato). */
export interface RegistroMock {
  id: number;
  /** Solo en variantes: id del registro principal. */
  idPrincipal: number | null;
  imagen: string | null;
  imagenVersion: number;
  /** Registro oculto (`_auxiliar_`). */
  oculto: boolean;
  valores: Valores;
  /** nombre de campo multidato → valores. */
  multidatos: Record<string, string[]>;
}

/** Estado mutable completo de un álbum mock. */
export interface AlbumMock {
  albumId: number;
  ruta: string;
  nombre: string;
  campos: CampoDef[];
  principal: RegistroMock[];
  variantes: RegistroMock[];
  grupos: Grupo[];
  /** `${campoId}:${principal}` → categorías. */
  categorias: Map<string, CategoriaVal[]>;
  /** nombre de filtro → condiciones. */
  filtros: Map<string, CondicionFiltro[]>;
  proximoId: number;
  proximoCampoId: number;
  proximoGrupoId: number;
}

/** Generador congruencial lineal — determinista entre recargas. */
function crearRng(semilla: number): () => number {
  let s = semilla >>> 0;
  return () => {
    s = (s * 1664525 + 1013904223) >>> 0;
    return s / 0xffffffff;
  };
}

const MARCAS = ["Acme", "Lumina", "Vértice", "Nórdika", "Solar", "Pampa"];
const LINEAS = ["Hogar", "Oficina", "Exterior", "Infantil", "Premium"];
const SUSTANTIVOS = [
  "Lámpara", "Silla", "Mesa", "Florero", "Reloj", "Espejo", "Tapete",
  "Cuadro", "Banco", "Estante", "Cojín", "Maceta", "Perchero", "Frutero",
];
const ADJETIVOS = [
  "clásico", "moderno", "artesanal", "plegable", "vintage", "minimalista",
  "rústico", "esmaltado", "tejido", "reciclado",
];
const ETIQUETAS = [
  "nuevo", "oferta", "importado", "frágil", "premium", "eco", "edición limitada",
];
const TALLAS = ["CH", "M", "G", "XG"];
const COLORES = ["Rojo", "Azul", "Verde", "Natural", "Negro", "Blanco"];

/** Crea las definiciones de campos del álbum demo. */
function camposDemo(): CampoDef[] {
  const base = {
    decimales: 0,
    totalizable: false,
    formula: null as string | null,
    visible: true,
    modificable: true,
    formato: null as CampoDef["formato"],
  };
  let orden = 0;
  const def = (
    id: number,
    nombre: string,
    tabla: "principal" | "variantes",
    tipo: CampoDef["tipo"],
    extra: Partial<CampoDef> = {},
  ): CampoDef => ({
    id,
    nombre,
    colFisica: `f_${id}`,
    tabla,
    tipo,
    ...base,
    ordenVisible: orden++,
    ...extra,
  });
  return [
    def(1, "Clave", "principal", "texto"),
    def(2, "Nombre", "principal", "texto"),
    def(3, "Marca", "principal", "texto"),
    def(4, "Línea", "principal", "texto"),
    def(5, "Cantidad", "principal", "numerico", { totalizable: true }),
    def(6, "Precio", "principal", "moneda", { decimales: 2, totalizable: true }),
    def(7, "Fecha Alta", "principal", "fecha"),
    def(8, "Importe", "principal", "calculado", {
      decimales: 2,
      totalizable: true,
      formula: "Cantidad * Precio",
      modificable: false,
    }),
    def(9, "Etiquetas", "principal", "multidato"),
    def(10, "Descripción", "principal", "texto"),
    def(15, "Avance", "principal", "calculado", {
      decimales: 2,
      formula: "100 * Cantidad / 120",
      formato: "porcentaje",
      modificable: false,
    }),
    def(11, "Talla", "variantes", "texto"),
    def(12, "Color", "variantes", "texto"),
    def(13, "Existencia", "variantes", "numerico", { totalizable: true }),
    def(14, "Precio Var", "variantes", "moneda", { decimales: 2 }),
  ];
}

/** Construye el álbum demo completo (240 principales, ~150 variantes). */
export function crearAlbumDemo(albumId: number, ruta: string): AlbumMock {
  const rng = crearRng(20260104);
  const campos = camposDemo();
  const principal: RegistroMock[] = [];
  const variantes: RegistroMock[] = [];
  let id = 1;
  let idVar = 1;

  const elegir = <T>(arr: T[]): T => arr[Math.floor(rng() * arr.length)];

  for (let i = 0; i < 240; i++) {
    const marca = elegir(MARCAS);
    const linea = elegir(LINEAS);
    const nombre = `${elegir(SUSTANTIVOS)} ${elegir(ADJETIVOS)}`;
    const cantidad = Math.floor(rng() * 120);
    const precio = Math.round(rng() * 250000) / 100;
    const dia = 1 + Math.floor(rng() * 28);
    const mes = 1 + Math.floor(rng() * 12);
    const anio = 2019 + Math.floor(rng() * 8);
    const nEtiquetas = Math.floor(rng() * 3);
    const etiquetas: string[] = [];
    for (let e = 0; e < nEtiquetas; e++) {
      const et = elegir(ETIQUETAS);
      if (!etiquetas.includes(et)) etiquetas.push(et);
    }
    const reg: RegistroMock = {
      id,
      idPrincipal: null,
      imagen: `imagenes/demo_${id}.jpg`,
      imagenVersion: 1,
      // Unos pocos registros ocultos para probar "Mostrar ocultos".
      oculto: i % 40 === 7,
      valores: {
        Clave: `SKU-${String(1000 + id)}`,
        Nombre: nombre,
        Marca: marca,
        "Línea": linea,
        Cantidad: cantidad,
        Precio: precio,
        "Fecha Alta": `${anio}-${String(mes).padStart(2, "0")}-${String(dia).padStart(2, "0")}`,
        Importe: Math.round(cantidad * precio * 100) / 100,
        Avance: Math.round((100 * cantidad) / 120 * 100) / 100,
        Etiquetas: etiquetas.length,
        "Descripción": `${nombre} de la línea ${linea}, marca ${marca}.`,
      },
      multidatos: { Etiquetas: etiquetas },
    };
    principal.push(reg);

    // 1 de cada 4 registros tiene 2-4 variantes (talla/color).
    if (i % 4 === 0) {
      const n = 2 + Math.floor(rng() * 3);
      for (let v = 0; v < n; v++) {
        variantes.push({
          id: idVar,
          idPrincipal: id,
          imagen: rng() > 0.4 ? `imagenes/var_${idVar}.jpg` : null,
          imagenVersion: 1,
          oculto: false,
          valores: {
            Talla: elegir(TALLAS),
            Color: elegir(COLORES),
            Existencia: Math.floor(rng() * 40),
            "Precio Var": Math.round(precio * (0.9 + rng() * 0.3) * 100) / 100,
          },
          multidatos: {},
        });
        idVar++;
      }
    }
    id++;
  }

  const categorias = new Map<string, CategoriaVal[]>();
  categorias.set(
    "9:true",
    ETIQUETAS.map((valor, i) => ({ valor, esDefault: i < 3 })),
  );

  const filtros = new Map<string, CondicionFiltro[]>();
  filtros.set("Caros (>$1,000)", [
    { opRel: null, campo: "Precio", opComp: "mayor", valor: "1000" },
  ]);
  filtros.set("Acme en oferta", [
    { opRel: null, campo: "Marca", opComp: "igual", valor: "Acme" },
    { opRel: "y", campo: "Etiquetas", opComp: "contiene", valor: "oferta" },
  ]);

  return {
    albumId,
    ruta,
    nombre: "Catálogo Demo",
    campos,
    principal,
    variantes,
    grupos: [
      { id: 1, nombre: "Por Marca", por: "Marca", luego1: null, luego2: null },
      { id: 2, nombre: "Marca y Línea", por: "Marca", luego1: "Línea", luego2: null },
      { id: 3, nombre: "Línea / Marca / Fecha", por: "Línea", luego1: "Marca", luego2: "Fecha Alta" },
    ],
    categorias,
    filtros,
    proximoId: id,
    proximoCampoId: 16,
    proximoGrupoId: 4,
  };
}
