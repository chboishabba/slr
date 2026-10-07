//! Matter-indexed discovery for persisted controversy packets.
//!
//! Workbench consumers select by the active persisted Matter rather than an
//! environment variable or caller-authored controversy manifest. Every row is
//! reopened through the canonical controversy loader before it is returned.

use postgres::{Client, NoTls};

use crate::{
    load_matter_controversy, DatabaseConfig, MatterControversyError,
    PersistedMatterControversy,
};

pub fn load_matter_controversies_for_matter(
    config: &DatabaseConfig,
    matter_ref: &str,
) -> Result<Vec<PersistedMatterControversy>, MatterControversyError> {
    if matter_ref.trim().is_empty() {
        return Err(MatterControversyError::EmptyCoordinate("matter_ref"));
    }
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let rows = client.query(
        r#"SELECT controversy_ref
           FROM semantic.matter_controversy
           WHERE matter_ref=$1
           ORDER BY controversy_ref"#,
        &[&matter_ref],
    )?;
    let mut values = Vec::with_capacity(rows.len());
    for row in rows {
        let controversy_ref: String = row.get(0);
        let value = load_matter_controversy(config, &controversy_ref)?;
        if value.controversy.matter_ref != matter_ref {
            return Err(MatterControversyError::WrongMatterOwner);
        }
        values.push(value);
    }
    Ok(values)
}
