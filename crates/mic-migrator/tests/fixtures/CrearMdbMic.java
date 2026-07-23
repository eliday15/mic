// Crea un .mdb Jet4 (Access 2000) con el esquema EXACTO de un álbum del MIC
// clásico (VB6, db.bas): propiedades, Principal, Variantes, Multidatos,
// Categorias — con datos en español (acentos, ñ) como los reales.
import com.healthmarketscience.jackcess.*;
import java.io.File;
import java.util.Date;

public class CrearMdbMic {
    public static void main(String[] args) throws Exception {
        File f = new File(args.length > 0 ? args[0] : "/tmp/mic-album.mdb");
        if (f.exists()) f.delete();
        Database db = DatabaseBuilder.create(Database.FileFormat.V2000, f);

        // --- propiedades (metadatos de campos, columnas exactas de db.bas) ---
        Table props = new TableBuilder("propiedades")
            .addColumn(new ColumnBuilder("Nombre", DataType.TEXT))
            .addColumn(new ColumnBuilder("Tipo", DataType.BYTE))
            .addColumn(new ColumnBuilder("longitud", DataType.BYTE))
            .addColumn(new ColumnBuilder("decimales", DataType.BYTE))
            .addColumn(new ColumnBuilder("totalizable", DataType.BOOLEAN))
            .addColumn(new ColumnBuilder("sInfo", DataType.TEXT))
            .addColumn(new ColumnBuilder("TipoSal", DataType.BYTE))
            .addColumn(new ColumnBuilder("Enprincipal", DataType.BOOLEAN))
            .addColumn(new ColumnBuilder("Modificable", DataType.BOOLEAN))
            .addColumn(new ColumnBuilder("Visible", DataType.BOOLEAN))
            .addColumn(new ColumnBuilder("OrdenVisible", DataType.BYTE))
            .toTable(db);
        // sistema
        props.addRow("_imagen_", (byte)0, (byte)255, (byte)0, false, " ", (byte)0, true, true, false, (byte)0);
        props.addRow("_id_", (byte)1, (byte)0, (byte)0, false, " ", (byte)0, true, false, false, (byte)0);
        props.addRow("_auxiliar_", (byte)1, (byte)0, (byte)0, false, " ", (byte)0, true, false, false, (byte)0);
        props.addRow("_variantes_", (byte)1, (byte)0, (byte)0, false, " ", (byte)0, true, false, false, (byte)0);
        // usuario (principal)
        props.addRow("Clave", (byte)0, (byte)50, (byte)0, false, " ", (byte)0, true, true, true, (byte)1);
        props.addRow("Descripción", (byte)0, (byte)255, (byte)0, false, " ", (byte)0, true, true, true, (byte)2);
        props.addRow("Cantidad", (byte)1, (byte)10, (byte)0, true, " ", (byte)0, true, true, true, (byte)3);
        props.addRow("Precio", (byte)2, (byte)10, (byte)2, true, " ", (byte)0, true, true, true, (byte)4);
        props.addRow("Fecha Alta", (byte)3, (byte)0, (byte)0, false, " ", (byte)0, true, true, true, (byte)5);
        props.addRow("Importe", (byte)4, (byte)0, (byte)2, true, "Cantidad*Precio", (byte)1, true, false, true, (byte)6);
        props.addRow("Etiquetas", (byte)5, (byte)0, (byte)0, false, " ", (byte)0, true, true, true, (byte)7);
        // usuario (variantes)
        props.addRow("Talla", (byte)0, (byte)20, (byte)0, false, " ", (byte)0, false, true, true, (byte)1);
        props.addRow("Color", (byte)0, (byte)30, (byte)0, false, " ", (byte)0, false, true, false, (byte)0);

        // --- Principal ---
        Table ppal = new TableBuilder("Principal")
            .addColumn(new ColumnBuilder("Clave", DataType.TEXT))
            .addColumn(new ColumnBuilder("Descripción", DataType.MEMO))
            .addColumn(new ColumnBuilder("Cantidad", DataType.DOUBLE))
            .addColumn(new ColumnBuilder("Precio", DataType.MONEY))
            .addColumn(new ColumnBuilder("Fecha Alta", DataType.SHORT_DATE_TIME))
            .addColumn(new ColumnBuilder("Importe", DataType.DOUBLE))
            .addColumn(new ColumnBuilder("Etiquetas", DataType.TEXT))
            .addColumn(new ColumnBuilder("_imagen_", DataType.TEXT))
            .addColumn(new ColumnBuilder("_id_", DataType.LONG))
            .addColumn(new ColumnBuilder("_auxiliar_", DataType.BOOLEAN))
            .addColumn(new ColumnBuilder("_variantes_", DataType.BOOLEAN))
            .toTable(db);
        String[] descs = {"Cinturón de piel genuina", "Camisa mañanera de algodón",
            "Pantalón añejo estilo español", "Bolsa de diseño con corazón", "Zapato niño"};
        for (int i = 1; i <= 60; i++) {
            boolean conVar = (i % 5 == 1);
            ppal.addRow("ALM-" + String.format("%04d", i),
                descs[i % 5] + " número " + i + " — año " + (2000 + (i % 8)),
                (double)(i * 3), new java.math.BigDecimal(i * 17.50),
                new Date(100 + (i % 20), (i % 12), 1 + (i % 27)),
                (double)(i * 3) * (i * 17.50), null,
                "G:\\MIC\\imagenes\\alm" + i + ".jpg", i, false, conVar);
        }

        // --- Variantes ---
        Table vars = new TableBuilder("Variantes")
            .addColumn(new ColumnBuilder("Talla", DataType.TEXT))
            .addColumn(new ColumnBuilder("Color", DataType.TEXT))
            .addColumn(new ColumnBuilder("_imagen_", DataType.TEXT))
            .addColumn(new ColumnBuilder("_id_", DataType.LONG))
            .addColumn(new ColumnBuilder("_idprincipal_", DataType.LONG))
            .toTable(db);
        int vid = 1000;
        for (int i = 1; i <= 60; i += 5) {
            vars.addRow("CH", "Añil", "G:\\MIC\\imagenes\\v" + vid + ".jpg", vid++, i);
            vars.addRow("XG", "Café", "G:\\MIC\\imagenes\\v" + vid + ".jpg", vid++, i);
        }

        // --- Multidatos ---
        Table multi = new TableBuilder("Multidatos")
            .addColumn(new ColumnBuilder("Id", DataType.LONG))
            .addColumn(new ColumnBuilder("Principal", DataType.BOOLEAN))
            .addColumn(new ColumnBuilder("Campo_n", DataType.TEXT))
            .addColumn(new ColumnBuilder("Valor", DataType.TEXT))
            .toTable(db);
        for (int i = 1; i <= 60; i += 3) {
            multi.addRow(i, true, "Etiquetas", "promoción");
            multi.addRow(i, true, "Etiquetas", "línea clásica");
        }

        // --- Categorias ---
        Table cats = new TableBuilder("Categorias")
            .addColumn(new ColumnBuilder("Campo_n", DataType.TEXT))
            .addColumn(new ColumnBuilder("Principal", DataType.BOOLEAN))
            .addColumn(new ColumnBuilder("Valor", DataType.TEXT))
            .addColumn(new ColumnBuilder("Default", DataType.BOOLEAN))
            .toTable(db);
        cats.addRow("Etiquetas", true, "promoción", true);
        cats.addRow("Etiquetas", true, "línea clásica", false);
        cats.addRow("Etiquetas", true, "descontinuado", false);

        db.close();
        System.out.println("creado: " + f + " (" + f.length() + " bytes)");
    }
}
