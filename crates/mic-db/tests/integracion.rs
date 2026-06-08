//! Tests de integración de la capa SQLite de MIC 3.0.
//!
//! Cubren: creación de álbum + campos de cada tipo, CRUD de registros, consulta
//! con filtros + orden de 3 niveles, FTS sin acentos, multidatos con conteo,
//! grupos con árbol de conteos, y un benchmark de 50k registros (ignorado por
//! defecto, se corre en release con `--ignored`).

use std::collections::HashMap;

use mic_core::calc::MotorCalculo;
use mic_core::model::{
    CampoNuevo, CondicionFiltro, Direccion, Grupo, OpComp, OpRel, OrdenCampo, QueryReq,
    SeleccionGrupo, Tabla, TipoCampo, Valor, Valores,
};
use mic_db::{
    pool::AlbumDb, repo_campos, repo_categorias, repo_filtros, repo_grupos, repo_multidatos,
    repo_registros,
};

/// Crea un álbum temporal en un directorio temporal y devuelve (db, tempdir).
fn album_tmp() -> (AlbumDb, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let ruta = dir.path().join("prueba.micdb");
    let db = AlbumDb::crear(&ruta, "Prueba").unwrap();
    (db, dir)
}

/// Atajo para construir un CampoNuevo principal.
fn campo(nombre: &str, tipo: TipoCampo) -> CampoNuevo {
    CampoNuevo {
        nombre: nombre.to_string(),
        tabla: Tabla::Principal,
        tipo,
        decimales: 2,
        totalizable: false,
        formula: None,
        visible: true,
        modificable: true,
        orden_visible: 0,
        formato: None,
    }
}

fn val_txt(s: &str) -> Valor {
    Valor::Texto(s.to_string())
}
fn val_num(n: f64) -> Valor {
    Valor::Numero(n)
}

#[test]
fn crea_album_y_reabre_con_pragmas() {
    let dir = tempfile::tempdir().unwrap();
    let ruta = dir.path().join("a.micdb");
    let db = AlbumDb::crear(&ruta, "Album1").unwrap();
    assert!(ruta.exists());
    assert!(db.dir_imagenes().ends_with("imagenes"));

    // Reabrir.
    drop(db);
    let db2 = AlbumDb::abrir(&ruta).unwrap();
    let conn = db2.conn().unwrap();
    // PRAGMAs activos.
    let fk: i64 = conn
        .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
        .unwrap();
    assert_eq!(fk, 1);
    let jm: String = conn
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .unwrap();
    assert_eq!(jm.to_lowercase(), "wal");
    // Metadatos.
    let nombre: String = conn
        .query_row("SELECT valor FROM mic_album WHERE clave='nombre'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(nombre, "Album1");
}

#[test]
fn crea_campos_de_cada_tipo() {
    let (db, _d) = album_tmp();
    let conn = db.conn().unwrap();

    let tipos = [
        ("Titulo", TipoCampo::Texto),
        ("Precio", TipoCampo::Moneda),
        ("Cantidad", TipoCampo::Numerico),
        ("Vencimiento", TipoCampo::Fecha),
        ("Etiquetas", TipoCampo::Multidato),
    ];
    for (n, t) in tipos {
        let def = repo_campos::crear(&conn, &campo(n, t)).unwrap();
        assert_eq!(def.col_fisica, format!("f_{}", def.id));
    }
    // Campo calculado que referencia otros.
    let mut calc = campo("Total", TipoCampo::Calculado);
    calc.formula = Some("Precio * Cantidad".to_string());
    repo_campos::crear(&conn, &calc).unwrap();

    let campos = repo_campos::listar(&conn).unwrap();
    assert_eq!(campos.len(), 6);

    // La columna física existe en principal.
    let cols: Vec<String> = {
        let mut stmt = conn.prepare("PRAGMA table_info(principal)").unwrap();
        stmt.query_map([], |r| r.get::<_, String>(1))
            .unwrap()
            .map(|x| x.unwrap())
            .collect()
    };
    for c in &campos {
        assert!(cols.contains(&c.col_fisica), "falta {}", c.col_fisica);
    }
}

#[test]
fn crud_registros_con_calculados() {
    let (db, _d) = album_tmp();
    let mut conn = db.conn().unwrap();

    repo_campos::crear(&conn, &campo("Titulo", TipoCampo::Texto)).unwrap();
    repo_campos::crear(&conn, &campo("Precio", TipoCampo::Moneda)).unwrap();
    repo_campos::crear(&conn, &campo("Cantidad", TipoCampo::Numerico)).unwrap();
    let mut calc = campo("Total", TipoCampo::Calculado);
    calc.formula = Some("Precio * Cantidad".to_string());
    repo_campos::crear(&conn, &calc).unwrap();

    let campos = repo_campos::listar(&conn).unwrap();
    let motor = MotorCalculo::new(&campos).unwrap();

    let mut v: Valores = HashMap::new();
    v.insert("Titulo".into(), val_txt("Café molido"));
    v.insert("Precio".into(), val_num(10.0));
    v.insert("Cantidad".into(), val_num(3.0));

    let id = repo_registros::crear(
        &mut conn,
        &campos,
        Some(&motor),
        Tabla::Principal,
        &v,
        &HashMap::new(),
        None,
        None,
        None,
    )
    .unwrap();

    let reg = repo_registros::obtener(&conn, &campos, Tabla::Principal, id).unwrap();
    assert_eq!(reg.valores.get("Titulo").unwrap().como_texto(), "Café molido");
    // Calculado persistido = 30.
    assert_eq!(reg.valores.get("Total").unwrap().como_f64().unwrap(), 30.0);

    // Editar: cambiar cantidad → recalcula total.
    let mut v2 = reg.valores.clone();
    v2.insert("Cantidad".into(), val_num(5.0));
    let reg2 = repo_registros::editar(
        &mut conn,
        &campos,
        Some(&motor),
        Tabla::Principal,
        id,
        &v2,
        None,
    )
    .unwrap();
    assert_eq!(reg2.valores.get("Total").unwrap().como_f64().unwrap(), 50.0);

    // Total de registros.
    assert_eq!(repo_registros::total(&conn).unwrap(), 1);

    // Eliminar.
    repo_registros::eliminar(&mut conn, Tabla::Principal, &[id]).unwrap();
    assert_eq!(repo_registros::total(&conn).unwrap(), 0);
}

#[test]
fn query_con_filtros_y_orden_3_niveles() {
    let (db, _d) = album_tmp();
    let mut conn = db.conn().unwrap();

    repo_campos::crear(&conn, &campo("Categoria", TipoCampo::Texto)).unwrap();
    repo_campos::crear(&conn, &campo("Marca", TipoCampo::Texto)).unwrap();
    repo_campos::crear(&conn, &campo("Precio", TipoCampo::Moneda)).unwrap();
    let campos = repo_campos::listar(&conn).unwrap();

    let datos = [
        ("A", "X", 30.0),
        ("A", "X", 10.0),
        ("A", "Y", 20.0),
        ("B", "X", 5.0),
        ("B", "Z", 99.0),
    ];
    for (cat, marca, precio) in datos {
        let mut v: Valores = HashMap::new();
        v.insert("Categoria".into(), val_txt(cat));
        v.insert("Marca".into(), val_txt(marca));
        v.insert("Precio".into(), val_num(precio));
        repo_registros::crear(
            &mut conn,
            &campos,
            None,
            Tabla::Principal,
            &v,
            &HashMap::new(),
            None,
            None,
            None,
        )
        .unwrap();
    }

    // Filtro: Categoria = A, orden Marca asc, Precio asc.
    let req = QueryReq {
        tabla: Tabla::Principal,
        id_principal: None,
        grupo: None,
        filtro_rapido: None,
        condiciones: vec![CondicionFiltro {
            op_rel: None,
            campo: "Categoria".into(),
            op_comp: OpComp::Igual,
            valor: "A".into(),
        }],
        busqueda: None,
        orden: vec![
            OrdenCampo {
                campo: "Marca".into(),
                direccion: Direccion::Asc,
            },
            OrdenCampo {
                campo: "Precio".into(),
                direccion: Direccion::Asc,
            },
            OrdenCampo {
                campo: "Categoria".into(),
                direccion: Direccion::Asc,
            },
        ],
        incluir_ocultos: false,
        offset: 0,
        limit: 100,
    };
    let page = repo_registros::query(&conn, &campos, &req).unwrap();
    assert_eq!(page.total, 3);
    // Orden esperado: (X,10),(X,30),(Y,20).
    let precios: Vec<f64> = page
        .registros
        .iter()
        .map(|r| r.valores.get("Precio").unwrap().como_f64().unwrap())
        .collect();
    assert_eq!(precios, vec![10.0, 30.0, 20.0]);

    // Filtro con OR: Precio > 50 O Categoria = B.
    let req_or = QueryReq {
        tabla: Tabla::Principal,
        id_principal: None,
        grupo: None,
        filtro_rapido: None,
        condiciones: vec![
            CondicionFiltro {
                op_rel: None,
                campo: "Precio".into(),
                op_comp: OpComp::Mayor,
                valor: "50".into(),
            },
            CondicionFiltro {
                op_rel: Some(OpRel::O),
                campo: "Categoria".into(),
                op_comp: OpComp::Igual,
                valor: "B".into(),
            },
        ],
        busqueda: None,
        orden: vec![],
        incluir_ocultos: false,
        offset: 0,
        limit: 100,
    };
    let page_or = repo_registros::query(&conn, &campos, &req_or).unwrap();
    // B/X/5, B/Z/99 (ambos B) → 2.
    assert_eq!(page_or.total, 2);
}

#[test]
fn fts_busca_sin_acentos() {
    let (db, _d) = album_tmp();
    let mut conn = db.conn().unwrap();

    repo_campos::crear(&conn, &campo("Nombre", TipoCampo::Texto)).unwrap();
    let campos = repo_campos::listar(&conn).unwrap();

    for nombre in ["Café Especial", "Té Verde", "Cacao"] {
        let mut v: Valores = HashMap::new();
        v.insert("Nombre".into(), val_txt(nombre));
        repo_registros::crear(
            &mut conn,
            &campos,
            None,
            Tabla::Principal,
            &v,
            &HashMap::new(),
            None,
            None,
            None,
        )
        .unwrap();
    }

    // "cafe" (sin acento) encuentra "Café".
    let req = QueryReq {
        tabla: Tabla::Principal,
        id_principal: None,
        grupo: None,
        filtro_rapido: None,
        condiciones: vec![],
        busqueda: Some("cafe".into()),
        orden: vec![],
        incluir_ocultos: false,
        offset: 0,
        limit: 100,
    };
    let page = repo_registros::query(&conn, &campos, &req).unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(
        page.registros[0].valores.get("Nombre").unwrap().como_texto(),
        "Café Especial"
    );
}

#[test]
fn multidatos_y_conteo() {
    let (db, _d) = album_tmp();
    let mut conn = db.conn().unwrap();

    repo_campos::crear(&conn, &campo("Nombre", TipoCampo::Texto)).unwrap();
    repo_campos::crear(&conn, &campo("Etiquetas", TipoCampo::Multidato)).unwrap();
    let campos = repo_campos::listar(&conn).unwrap();
    let campo_etq = campos.iter().find(|c| c.nombre == "Etiquetas").unwrap();

    let mut v: Valores = HashMap::new();
    v.insert("Nombre".into(), val_txt("Producto"));
    let mut multi: HashMap<String, Vec<String>> = HashMap::new();
    multi.insert(
        "Etiquetas".into(),
        vec!["rojo".into(), "grande".into(), "oferta".into()],
    );

    let id = repo_registros::crear(
        &mut conn,
        &campos,
        None,
        Tabla::Principal,
        &v,
        &multi,
        None,
        None,
        None,
    )
    .unwrap();

    // Conteo en la columna física = 3.
    let reg = repo_registros::obtener(&conn, &campos, Tabla::Principal, id).unwrap();
    assert_eq!(
        reg.valores.get("Etiquetas").unwrap().como_f64().unwrap(),
        3.0
    );
    assert_eq!(reg.multidatos.get("Etiquetas").unwrap().len(), 3);

    // Listado directo.
    let lista = repo_multidatos::listar(&conn, id, campo_etq.id, true).unwrap();
    assert_eq!(lista, vec!["rojo", "grande", "oferta"]);

    // Categorías alimentadas para autocomplete.
    let sug = repo_categorias::sugerir(&conn, campo_etq.id, true, "ro", 20).unwrap();
    assert_eq!(sug, vec!["rojo"]);

    // Filtro rápido por multidato (EXISTS).
    let req = QueryReq {
        tabla: Tabla::Principal,
        id_principal: None,
        grupo: None,
        filtro_rapido: Some(mic_core::model::FiltroRapido {
            campo: "Etiquetas".into(),
            valor: "grande".into(),
        }),
        condiciones: vec![],
        busqueda: None,
        orden: vec![],
        incluir_ocultos: false,
        offset: 0,
        limit: 100,
    };
    assert_eq!(repo_registros::query(&conn, &campos, &req).unwrap().total, 1);

    // FTS también indexa multidatos.
    let req_fts = QueryReq {
        tabla: Tabla::Principal,
        id_principal: None,
        grupo: None,
        filtro_rapido: None,
        condiciones: vec![],
        busqueda: Some("oferta".into()),
        orden: vec![],
        incluir_ocultos: false,
        offset: 0,
        limit: 100,
    };
    assert_eq!(
        repo_registros::query(&conn, &campos, &req_fts).unwrap().total,
        1
    );
}

#[test]
fn grupos_arbol_con_conteos() {
    let (db, _d) = album_tmp();
    let mut conn = db.conn().unwrap();

    repo_campos::crear(&conn, &campo("Pais", TipoCampo::Texto)).unwrap();
    repo_campos::crear(&conn, &campo("Ciudad", TipoCampo::Texto)).unwrap();
    let campos = repo_campos::listar(&conn).unwrap();

    let datos = [
        ("Mexico", "CDMX"),
        ("Mexico", "CDMX"),
        ("Mexico", "Monterrey"),
        ("Chile", "Santiago"),
    ];
    for (p, c) in datos {
        let mut v: Valores = HashMap::new();
        v.insert("Pais".into(), val_txt(p));
        v.insert("Ciudad".into(), val_txt(c));
        repo_registros::crear(
            &mut conn,
            &campos,
            None,
            Tabla::Principal,
            &v,
            &HashMap::new(),
            None,
            None,
            None,
        )
        .unwrap();
    }

    let gid = repo_grupos::guardar(
        &conn,
        &Grupo {
            id: 0,
            nombre: "Geografia".into(),
            por: "Pais".into(),
            luego1: Some("Ciudad".into()),
            luego2: None,
        },
    )
    .unwrap();

    let arbol = repo_grupos::arbol(&conn, &campos, gid).unwrap();
    // Dos países: Chile (1), Mexico (3) — orden alfabético NOCASE.
    assert_eq!(arbol.len(), 2);
    let mexico = arbol.iter().find(|n| n.valor == "Mexico").unwrap();
    assert_eq!(mexico.conteo, 3);
    // Ciudades de Mexico: CDMX(2), Monterrey(1).
    assert_eq!(mexico.hijos.len(), 2);
    let cdmx = mexico.hijos.iter().find(|n| n.valor == "CDMX").unwrap();
    assert_eq!(cdmx.conteo, 2);

    // Selección de grupo nivel 1 = Mexico → filtra a 3.
    let req = QueryReq {
        tabla: Tabla::Principal,
        id_principal: None,
        grupo: Some(SeleccionGrupo {
            grupo_id: gid,
            valores: vec![Some("Mexico".into())],
        }),
        filtro_rapido: None,
        condiciones: vec![],
        busqueda: None,
        orden: vec![],
        incluir_ocultos: false,
        offset: 0,
        limit: 100,
    };
    assert_eq!(repo_registros::query(&conn, &campos, &req).unwrap().total, 3);
}

#[test]
fn variantes_ligadas_a_principal() {
    let (db, _d) = album_tmp();
    let mut conn = db.conn().unwrap();

    repo_campos::crear(&conn, &campo("Modelo", TipoCampo::Texto)).unwrap();
    // Campo de variante.
    let mut talla = campo("Talla", TipoCampo::Texto);
    talla.tabla = Tabla::Variantes;
    repo_campos::crear(&conn, &talla).unwrap();
    let campos = repo_campos::listar(&conn).unwrap();

    let mut v: Valores = HashMap::new();
    v.insert("Modelo".into(), val_txt("Camiseta"));
    let idp = repo_registros::crear(
        &mut conn,
        &campos,
        None,
        Tabla::Principal,
        &v,
        &HashMap::new(),
        None,
        None,
        None,
    )
    .unwrap();

    for talla in ["S", "M", "L"] {
        let mut vv: Valores = HashMap::new();
        vv.insert("Talla".into(), val_txt(talla));
        repo_registros::crear(
            &mut conn,
            &campos,
            None,
            Tabla::Variantes,
            &vv,
            &HashMap::new(),
            None,
            Some(idp),
            None,
        )
        .unwrap();
    }

    let vars = repo_registros::variantes_de(&conn, &campos, idp).unwrap();
    assert_eq!(vars.len(), 3);

    // El principal quedó marcado con _variantes_ = 1.
    let req = QueryReq {
        tabla: Tabla::Principal,
        id_principal: None,
        grupo: None,
        filtro_rapido: None,
        condiciones: vec![],
        busqueda: None,
        orden: vec![],
        incluir_ocultos: false,
        offset: 0,
        limit: 10,
    };
    let page = repo_registros::query(&conn, &campos, &req).unwrap();
    assert!(page.registros[0].tiene_variantes);

    // Borrar principal → cascada borra variantes.
    repo_registros::eliminar(&mut conn, Tabla::Principal, &[idp]).unwrap();
    let n: i64 = conn
        .query_row("SELECT COUNT(*) FROM variantes", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 0);
}

#[test]
fn editar_campo_cambia_tipo_convierte_datos() {
    let (db, _d) = album_tmp();
    let mut conn = db.conn().unwrap();

    let def = repo_campos::crear(&conn, &campo("Dato", TipoCampo::Texto)).unwrap();
    let campos = repo_campos::listar(&conn).unwrap();

    for s in ["10", "abc", "20"] {
        let mut v: Valores = HashMap::new();
        v.insert("Dato".into(), val_txt(s));
        repo_registros::crear(
            &mut conn,
            &campos,
            None,
            Tabla::Principal,
            &v,
            &HashMap::new(),
            None,
            None,
            None,
        )
        .unwrap();
    }

    // Texto → Numérico: "abc" pasa a 0 (paridad con CvrteCmps).
    let mut nuevo = campo("Dato", TipoCampo::Numerico);
    nuevo.nombre = "Dato".into();
    repo_campos::editar(&conn, def.id, &nuevo).unwrap();

    let suma: f64 = conn
        .query_row(&format!("SELECT SUM({}) FROM principal", def.col_fisica), [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(suma, 30.0); // 10 + 0 + 20
}

#[test]
fn filtros_guardados_roundtrip() {
    let (db, _d) = album_tmp();
    let mut conn = db.conn().unwrap();

    let condiciones = vec![
        CondicionFiltro {
            op_rel: None,
            campo: "Precio".into(),
            op_comp: OpComp::Mayor,
            valor: "100".into(),
        },
        CondicionFiltro {
            op_rel: Some(OpRel::Y),
            campo: "Categoria".into(),
            op_comp: OpComp::Igual,
            valor: "A".into(),
        },
    ];
    repo_filtros::guardar(&mut conn, "Caros A", &condiciones).unwrap();

    assert_eq!(repo_filtros::listar(&conn).unwrap(), vec!["Caros A"]);
    let recuperado = repo_filtros::obtener(&conn, "Caros A").unwrap();
    assert_eq!(recuperado.len(), 2);
    assert_eq!(recuperado[1].op_rel, Some(OpRel::Y));
    assert_eq!(recuperado[1].op_comp, OpComp::Igual);

    repo_filtros::eliminar(&conn, "Caros A").unwrap();
    assert!(repo_filtros::listar(&conn).unwrap().is_empty());
}

/// Benchmark: 50_000 registros, query con filtro + orden de 3 niveles < 100ms.
/// Ignorado por defecto (lento en debug); correr con:
///   cargo test -p mic-db --release -- --ignored bench_50k
#[test]
#[ignore = "benchmark de 50k; correr en release con --ignored"]
fn bench_50k_query_filtro_orden() {
    use std::time::Instant;

    let (db, _d) = album_tmp();
    let mut conn = db.conn().unwrap();

    repo_campos::crear(&conn, &campo("Categoria", TipoCampo::Texto)).unwrap();
    repo_campos::crear(&conn, &campo("Marca", TipoCampo::Texto)).unwrap();
    repo_campos::crear(&conn, &campo("Precio", TipoCampo::Moneda)).unwrap();
    repo_campos::crear(&conn, &campo("Cantidad", TipoCampo::Numerico)).unwrap();
    let campos = repo_campos::listar(&conn).unwrap();

    // Siembra masiva en una sola transacción para rapidez.
    {
        let f_cat = &campos.iter().find(|c| c.nombre == "Categoria").unwrap().col_fisica;
        let f_marca = &campos.iter().find(|c| c.nombre == "Marca").unwrap().col_fisica;
        let f_precio = &campos.iter().find(|c| c.nombre == "Precio").unwrap().col_fisica;
        let f_cant = &campos.iter().find(|c| c.nombre == "Cantidad").unwrap().col_fisica;
        let tx = conn.transaction().unwrap();
        {
            let sql = format!(
                "INSERT INTO principal ({f_cat}, {f_marca}, {f_precio}, {f_cant}) \
                 VALUES (?1, ?2, ?3, ?4)"
            );
            let mut stmt = tx.prepare(&sql).unwrap();
            for i in 0..50_000u32 {
                let cat = format!("Cat{}", i % 50);
                let marca = format!("Marca{}", i % 200);
                let precio = (i % 1000) as f64 + 0.5;
                let cant = (i % 30) as f64;
                stmt.execute(rusqlite::params![cat, marca, precio, cant])
                    .unwrap();
            }
        }
        tx.commit().unwrap();
    }
    assert_eq!(repo_registros::total(&conn).unwrap(), 50_000);

    let req = QueryReq {
        tabla: Tabla::Principal,
        id_principal: None,
        grupo: None,
        filtro_rapido: None,
        condiciones: vec![CondicionFiltro {
            op_rel: None,
            campo: "Categoria".into(),
            op_comp: OpComp::Igual,
            valor: "Cat7".into(),
        }],
        busqueda: None,
        orden: vec![
            OrdenCampo {
                campo: "Marca".into(),
                direccion: Direccion::Asc,
            },
            OrdenCampo {
                campo: "Precio".into(),
                direccion: Direccion::Desc,
            },
            OrdenCampo {
                campo: "Cantidad".into(),
                direccion: Direccion::Asc,
            },
        ],
        incluir_ocultos: false,
        offset: 0,
        limit: 100,
    };

    // Calentamiento (carga páginas en caché).
    let _ = repo_registros::query(&conn, &campos, &req).unwrap();

    let inicio = Instant::now();
    let page = repo_registros::query(&conn, &campos, &req).unwrap();
    let dur = inicio.elapsed();

    assert_eq!(page.total, 1000); // 50000 / 50 categorías
    assert_eq!(page.registros.len(), 100);
    println!("query 50k filtro+orden3: {:?}", dur);
    assert!(
        dur.as_millis() < 100,
        "la consulta tardó {:?} (>100ms)",
        dur
    );
}
