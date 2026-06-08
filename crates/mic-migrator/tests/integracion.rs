//! Pruebas de integración de mic-migrator.
//!
//! No hay `.mdb` de prueba en el repo (la migración real requiere mdbtools y un
//! Access). Aquí cubrimos lo que sí podemos verificar de extremo a extremo sin
//! mdbtools: el importador de plantillas `.xms` leyendo un archivo real escrito
//! en disco con codificación Windows-1252 (incluyendo acentos en los nombres de
//! campo).

use std::io::Write;

use mic_core::model::{Tabla, TipoCampo};
use mic_migrator::parse_xms;

/// Escribe `bytes` a un archivo temporal `.xms` y devuelve su handle.
fn xms_temporal(bytes: &[u8]) -> tempfile::NamedTempFile {
    let mut f = tempfile::Builder::new()
        .suffix(".xms")
        .tempfile()
        .expect("crear temporal");
    f.write_all(bytes).expect("escribir xms");
    f.flush().expect("flush");
    f
}

#[test]
fn parse_xms_desde_archivo_cp1252() {
    // Plantilla con nombres de campo acentuados, codificada en Windows-1252.
    // "Descripción" -> 'ó' = 0xF3 ; "Año" -> 'ñ' = 0xF1.
    let mut bytes: Vec<u8> = Vec::new();
    bytes.extend_from_slice(
        b"<?xml version=\"1.0\" encoding=\"iso-8859-1\"?>\n<mic Version=\"2.0\" Plantilla=\"normal\">\n  <Registro>\n",
    );
    // <Descripci\xf3n Tipo="0"/>  (ó = 0xF3)
    bytes.extend_from_slice(b"    <Descripci\xf3n Tipo=\"0\"/>\n");
    // <A\xf1o Tipo="1" Decimales="0"/>  (ñ = 0xF1)
    bytes.extend_from_slice(b"    <A\xf1o Tipo=\"1\"/>\n");
    bytes.extend_from_slice(
        b"    <Precio Tipo=\"2\" Decimales=\"2\" Totalizable=\"True\"/>\n  </Registro>\n</mic>\n",
    );

    let archivo = xms_temporal(&bytes);
    let campos = parse_xms(archivo.path()).expect("parsear xms");

    assert_eq!(campos.len(), 3);
    assert_eq!(campos[0].nombre, "Descripción");
    assert_eq!(campos[0].tipo, TipoCampo::Texto);
    assert_eq!(campos[0].tabla, Tabla::Principal);

    assert_eq!(campos[1].nombre, "Año");
    assert_eq!(campos[1].tipo, TipoCampo::Numerico);

    assert_eq!(campos[2].nombre, "Precio");
    assert_eq!(campos[2].tipo, TipoCampo::Moneda);
    assert_eq!(campos[2].decimales, 2);
    assert!(campos[2].totalizable);
}

#[test]
fn parse_xms_archivo_inexistente_es_error() {
    let r = parse_xms(std::path::Path::new("/no/existe/plantilla.xms"));
    assert!(r.is_err());
}
