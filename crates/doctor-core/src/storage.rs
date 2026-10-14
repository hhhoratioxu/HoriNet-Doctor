use crate::{
    models::{Report, Settings},
    validation, Error, Result,
};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub struct Store {
    root: PathBuf,
}
impl Store {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(root.join("history"))?;
        Ok(Self { root })
    }
    pub fn settings(&self) -> Result<Settings> {
        let path = self.root.join("settings.json");
        if !path.exists() {
            return Ok(Settings::default());
        }
        let s: Settings = serde_json::from_slice(&read_limited(&path)?)?;
        validation::settings(&s)?;
        Ok(s)
    }
    pub fn save_settings(&self, s: &Settings) -> Result<()> {
        validation::settings(s)?;
        atomic(
            &self.root.join("settings.json"),
            &serde_json::to_vec_pretty(s)?,
        )
    }
    fn path(&self, id: &str) -> Result<PathBuf> {
        let uuid =
            uuid::Uuid::parse_str(id).map_err(|_| Error::Invalid("invalid history ID".into()))?;
        Ok(self.root.join("history").join(format!("{uuid}.json")))
    }
    pub fn save(&self, report: &Report, retention: usize) -> Result<()> {
        if !(1..=500).contains(&retention) {
            return Err(Error::Invalid("invalid retention".into()));
        }
        atomic(&self.path(&report.id)?, &serde_json::to_vec(report)?)?;
        for old in self.list()?.into_iter().skip(retention) {
            self.delete(&old.id)?;
        }
        Ok(())
    }
    pub fn get(&self, id: &str) -> Result<Report> {
        Ok(serde_json::from_slice(&read_limited(&self.path(id)?)?)?)
    }
    pub fn list(&self) -> Result<Vec<Report>> {
        let mut reports = Vec::new();
        for entry in fs::read_dir(self.root.join("history"))?.take(2000) {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            if uuid::Uuid::parse_str(id).is_err() {
                continue;
            }
            match self.get(id) {
                Ok(r) => reports.push(r),
                Err(_) => tracing::warn!(
                    "Skipped an unreadable local history record; no diagnostic data logged"
                ),
            }
        }
        reports.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        Ok(reports)
    }
    pub fn delete(&self, id: &str) -> Result<()> {
        fs::remove_file(self.path(id)?)?;
        Ok(())
    }
    pub fn clear(&self) -> Result<()> {
        for entry in fs::read_dir(self.root.join("history"))? {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Some(id) = path.file_stem().and_then(|s| s.to_str()) {
                    if uuid::Uuid::parse_str(id).is_ok() {
                        self.delete(id)?;
                    }
                }
            }
        }
        Ok(())
    }
}
fn read_limited(path: &Path) -> Result<Vec<u8>> {
    if fs::metadata(path)?.len() > 8 * 1024 * 1024 {
        return Err(Error::Invalid("local record exceeds 8 MiB limit".into()));
    }
    Ok(fs::read(path)?)
}
fn atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    let tmp = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        fs::rename(&tmp, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(tmp);
    }
    result
}
