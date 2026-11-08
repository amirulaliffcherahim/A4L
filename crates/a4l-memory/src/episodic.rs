use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EpisodicError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Episode {
    pub id: i64,
    pub kind: String, // "failure_bundle" | "task_summary" | "decision"
    pub task_id: String,
    pub title: String,
    pub symptoms: String,
    pub resolution: String,
    pub created_at: String,
}

pub struct EpisodicStore {
    db_path: PathBuf,
}

impl EpisodicStore {
    pub fn open(db_path: impl AsRef<Path>) -> Result<Self, EpisodicError> {
        let db_path = db_path.as_ref().to_path_buf();
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&db_path)?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS episodes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                kind TEXT NOT NULL,
                task_id TEXT NOT NULL,
                title TEXT NOT NULL,
                symptoms TEXT NOT NULL,
                resolution TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_episodes_symptoms ON episodes(symptoms);
            CREATE INDEX IF NOT EXISTS idx_episodes_task_id ON episodes(task_id);
            "#,
        )?;

        Ok(Self { db_path })
    }

    fn conn(&self) -> Result<Connection, EpisodicError> {
        Ok(Connection::open(&self.db_path)?)
    }

    pub fn insert_episode(
        &self,
        kind: &str,
        task_id: &str,
        title: &str,
        symptoms: &str,
        resolution: &str,
    ) -> Result<i64, EpisodicError> {
        let conn = self.conn()?;
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO episodes (kind, task_id, title, symptoms, resolution, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![kind, task_id, title, symptoms, resolution, now],
        )?;

        Ok(conn.last_insert_rowid())
    }

    pub fn search_by_symptom(&self, keyword: &str) -> Result<Vec<Episode>, EpisodicError> {
        let conn = self.conn()?;
        let pattern = format!("%{}%", keyword.trim());

        let mut stmt = conn.prepare(
            "SELECT id, kind, task_id, title, symptoms, resolution, created_at FROM episodes WHERE symptoms LIKE ?1 OR title LIKE ?1 ORDER BY id DESC LIMIT 10",
        )?;

        let episode_iter = stmt.query_map([&pattern], |row| {
            Ok(Episode {
                id: row.get(0)?,
                kind: row.get(1)?,
                task_id: row.get(2)?,
                title: row.get(3)?,
                symptoms: row.get(4)?,
                resolution: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;

        let mut results = Vec::new();
        for ep in episode_iter {
            results.push(ep?);
        }

        Ok(results)
    }
}
