//! Introspezione di database **live** (PostgreSQL).
//!
//! Si connette via il driver `postgres` (puro Rust, NoTLS per l'MVP) e legge
//! `information_schema` per ricostruire tabelle, colonne, chiavi primarie e
//! foreign key nel [`crate::model::Table`]. Oracle live richiede l'Instant
//! Client nativo (non impacchettabile) ed è lasciato all'analisi del DDL su file.
//!
//! La logica di assemblaggio righe → modello è separata in [`assemble`] e testata
//! senza bisogno di un database.

use crate::model::{Column, ForeignKey, Table};
use crate::{Error, Result};
use postgres::{Client, NoTls};

/// Riga di colonna letta da information_schema.
pub struct ColRow {
    pub schema: String,
    pub table: String,
    pub column: String,
    pub data_type: String,
    pub nullable: bool,
}
/// Riga di chiave primaria.
pub struct PkRow {
    pub schema: String,
    pub table: String,
    pub column: String,
}
/// Riga di foreign key.
pub struct FkRow {
    pub schema: String,
    pub table: String,
    pub column: String,
    pub ref_table: String,
    pub ref_column: String,
}

const COL_SQL: &str = "SELECT table_schema, table_name, column_name, data_type, is_nullable \
     FROM information_schema.columns \
     WHERE table_schema NOT IN ('pg_catalog','information_schema') \
     ORDER BY table_schema, table_name, ordinal_position";

const PK_SQL: &str = "SELECT tc.table_schema, tc.table_name, kcu.column_name \
     FROM information_schema.table_constraints tc \
     JOIN information_schema.key_column_usage kcu \
       ON kcu.constraint_name = tc.constraint_name AND kcu.table_schema = tc.table_schema \
     WHERE tc.constraint_type = 'PRIMARY KEY'";

const FK_SQL: &str = "SELECT tc.table_schema, tc.table_name, kcu.column_name, \
            ccu.table_name AS ref_table, ccu.column_name AS ref_column \
     FROM information_schema.table_constraints tc \
     JOIN information_schema.key_column_usage kcu \
       ON kcu.constraint_name = tc.constraint_name AND kcu.table_schema = tc.table_schema \
     JOIN information_schema.constraint_column_usage ccu \
       ON ccu.constraint_name = tc.constraint_name \
     WHERE tc.constraint_type = 'FOREIGN KEY'";

/// Si connette a PostgreSQL e restituisce le tabelle introspezionate.
/// `schema`: se indicato, limita a quello (altrimenti tutti gli schemi utente).
pub fn introspect_postgres(dsn: &str, schema: Option<&str>) -> Result<Vec<Table>> {
    let mut client = Client::connect(dsn, NoTls).map_err(|e| Error::Db(e.to_string()))?;

    let cols = client
        .query(COL_SQL, &[])
        .map_err(|e| Error::Db(e.to_string()))?
        .into_iter()
        .map(|r| ColRow {
            schema: r.get(0),
            table: r.get(1),
            column: r.get(2),
            data_type: r.get(3),
            nullable: r.get::<_, String>(4) == "YES",
        })
        .collect::<Vec<_>>();

    let pks = client
        .query(PK_SQL, &[])
        .map_err(|e| Error::Db(e.to_string()))?
        .into_iter()
        .map(|r| PkRow {
            schema: r.get(0),
            table: r.get(1),
            column: r.get(2),
        })
        .collect::<Vec<_>>();

    let fks = client
        .query(FK_SQL, &[])
        .map_err(|e| Error::Db(e.to_string()))?
        .into_iter()
        .map(|r| FkRow {
            schema: r.get(0),
            table: r.get(1),
            column: r.get(2),
            ref_table: r.get(3),
            ref_column: r.get(4),
        })
        .collect::<Vec<_>>();

    let mut tables = assemble(cols, pks, fks);
    if let Some(s) = schema {
        tables.retain(|t| t.schema.as_deref() == Some(s));
    }
    Ok(tables)
}

/// Assembla righe grezze nel modello `Table` (logica pura, testabile).
pub fn assemble(cols: Vec<ColRow>, pks: Vec<PkRow>, fks: Vec<FkRow>) -> Vec<Table> {
    use std::collections::BTreeMap;
    let key = |s: &str, t: &str| format!("{s}.{t}");

    let mut map: BTreeMap<String, Table> = BTreeMap::new();
    for c in cols {
        let k = key(&c.schema, &c.table);
        let entry = map.entry(k).or_insert_with(|| Table {
            id: format!("table:{}", c.table.to_lowercase()),
            name: c.table.clone(),
            schema: Some(c.schema.clone()),
            columns: vec![],
            foreign_keys: vec![],
        });
        entry.columns.push(Column {
            name: c.column,
            data_type: c.data_type,
            nullable: c.nullable,
            primary_key: false,
        });
    }
    for pk in pks {
        if let Some(t) = map.get_mut(&key(&pk.schema, &pk.table)) {
            if let Some(col) = t.columns.iter_mut().find(|c| c.name == pk.column) {
                col.primary_key = true;
            }
        }
    }
    for fk in fks {
        if let Some(t) = map.get_mut(&key(&fk.schema, &fk.table)) {
            t.foreign_keys.push(ForeignKey {
                column: fk.column,
                references_table: fk.ref_table,
                references_column: fk.ref_column,
            });
        }
    }
    map.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assembla_tabelle_da_righe() {
        let cols = vec![
            ColRow { schema: "public".into(), table: "orders".into(), column: "id".into(), data_type: "integer".into(), nullable: false },
            ColRow { schema: "public".into(), table: "orders".into(), column: "customer_id".into(), data_type: "integer".into(), nullable: true },
        ];
        let pks = vec![PkRow { schema: "public".into(), table: "orders".into(), column: "id".into() }];
        let fks = vec![FkRow { schema: "public".into(), table: "orders".into(), column: "customer_id".into(), ref_table: "customers".into(), ref_column: "id".into() }];

        let tables = assemble(cols, pks, fks);
        assert_eq!(tables.len(), 1);
        let t = &tables[0];
        assert_eq!(t.id, "table:orders");
        assert!(t.columns.iter().find(|c| c.name == "id").unwrap().primary_key);
        assert_eq!(t.foreign_keys.len(), 1);
        assert_eq!(t.foreign_keys[0].references_table, "customers");
    }
}
