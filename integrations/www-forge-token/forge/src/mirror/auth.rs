//! Token storage: redb + serde_json per backend name.
use anyhow::{Context, Result};
use redb::{Database, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use std::path::Path;

const AUTH_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("auth_tokens");

/// Opaque token bundle stored per backend name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBundle {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>,
    pub extra: serde_json::Value,
}

pub struct AuthStore {
    db: Database,
}

impl AuthStore {
    pub fn open(forge_dir: &Path) -> Result<Self> {
        let path = forge_dir.join("auth.redb");
        let db = Database::create(&path).context("open auth.redb")?;
        {
            let write = db.begin_write()?;
            write.open_table(AUTH_TABLE)?;
            write.commit()?;
        }
        Ok(Self { db })
    }

    pub fn save(&self, backend: &str, bundle: &TokenBundle) -> Result<()> {
        let bytes = serde_json::to_vec(bundle)?;
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(AUTH_TABLE)?;
            table.insert(backend, bytes.as_slice())?;
        }
        txn.commit()?;
        Ok(())
    }

    pub fn load(&self, backend: &str) -> Result<Option<TokenBundle>> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(AUTH_TABLE)?;
        match table.get(backend)? {
            Some(v) => {
                let bundle: TokenBundle = serde_json::from_slice(v.value())?;
                Ok(Some(bundle))
            }
            None => Ok(None),
        }
    }

    pub fn list_backends(&self) -> Result<Vec<String>> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(AUTH_TABLE)?;
        let mut out = Vec::new();
        for entry in table.iter()? {
            let (k, _v) = entry?;
            out.push(k.value().to_string());
        }
        Ok(out)
    }
}
