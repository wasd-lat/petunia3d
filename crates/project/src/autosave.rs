//! Serviço puro de Autosave, Session Lock e Detecção de Recuperação (P3D-002).
//!
//! Contrato canônico:
//! 1. `Save explícito` -> projeto oficial (`project.petunia`);
//! 2. `Autosave` -> snapshots rotativos de recuperação (`.petunia/autosave/autosave-*.petunia`);
//! 3. Autosave **nunca** sobrescreve o arquivo oficial;
//! 4. Autosave **nunca** limpa o `dirty state` do documento;
//! 5. Retenção de $N$ snapshots (descarta o mais antigo ao exceder limite);
//! 6. Marcador de sessão (`session.lock`) para detecção determinística de unclean shutdown / crash;
//! 7. Recuperação segura: o usuário escolhe Recover, Open Saved ou Discard, sem sobrescrita silenciosa;
//! 8. 100% puro e desacoplado do egui (acionado por timestamps/relógio).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::Project;
use crate::format::{self, ProjectError};

/// Configurações do sistema de Autosave e Recuperação (P3D-002 §Configurações).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutosaveConfig {
    /// Habilita ou desabilita o autosave automático.
    pub enabled: bool,
    /// Intervalo entre snapshots em segundos (ex: 60s, 120s, 300s).
    pub interval_secs: u64,
    /// Quantidade máxima de snapshots retidos por projeto.
    pub keep_n: usize,
    /// Executa snapshot apenas quando o documento possuir modificações não salvas (dirty).
    pub only_when_dirty: bool,
}

impl Default for AutosaveConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_secs: 120, // 2 minutos
            keep_n: 5,
            only_when_dirty: true,
        }
    }
}

/// Metadados gravados no marcador de sessão (`session.lock`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionLockInfo {
    pub pid: u32,
    pub start_time: u64,
    pub project_name: String,
    pub project_path: Option<String>,
}

/// Informações diagnósticas para apresentação da decisão de recuperação (Recovery Dialog).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryInfo {
    pub snapshot_path: PathBuf,
    pub project_name: String,
    pub snapshot_time: u64,
    pub main_project_path: Option<PathBuf>,
    pub is_newer_than_main: bool,
}

/// Serviço de autosave e retenção de snapshots.
#[derive(Debug, Clone)]
pub struct AutosaveService {
    pub config: AutosaveConfig,
    pub last_autosave_time: u64,
    seq: u64,
}

impl Default for AutosaveService {
    fn default() -> Self {
        Self::new(AutosaveConfig::default())
    }
}

impl AutosaveService {
    pub fn new(config: AutosaveConfig) -> Self {
        Self {
            config,
            last_autosave_time: 0,
            seq: 0,
        }
    }

    /// Determina o diretório canônico de autosave para o projeto atual.
    pub fn autosave_dir(project_path: Option<&Path>) -> PathBuf {
        if let Some(path) = project_path {
            let parent = path.parent().unwrap_or_else(|| Path::new("."));
            parent.join(".petunia").join("autosave")
        } else {
            std::env::temp_dir()
                .join("petunia3d")
                .join("untitled_project")
                .join(".petunia")
                .join("autosave")
        }
    }

    /// Determina o caminho do marcador de sessão ativa (`session.lock`).
    pub fn session_lock_path(project_path: Option<&Path>) -> PathBuf {
        if let Some(path) = project_path {
            let parent = path.parent().unwrap_or_else(|| Path::new("."));
            parent.join(".petunia").join("session.lock")
        } else {
            std::env::temp_dir()
                .join("petunia3d")
                .join("untitled_project")
                .join(".petunia")
                .join("session.lock")
        }
    }

    /// Grava marcador de sessão ativa indicando que o projeto está aberto.
    pub fn create_session_lock(
        project_path: Option<&Path>,
        project_name: &str,
        start_time: u64,
    ) -> Result<PathBuf, std::io::Error> {
        let lock_path = Self::session_lock_path(project_path);
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let info = SessionLockInfo {
            pid: std::process::id(),
            start_time,
            project_name: project_name.to_string(),
            project_path: project_path.map(|p| p.to_string_lossy().to_string()),
        };
        let bytes = postcard::to_allocvec(&info)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        fs::write(&lock_path, bytes)?;
        Ok(lock_path)
    }

    /// Remove o marcador de sessão ativa em encerramento normal e limpo (clean shutdown).
    pub fn remove_session_lock(project_path: Option<&Path>) {
        let lock_path = Self::session_lock_path(project_path);
        let _ = fs::remove_file(lock_path);
    }

    /// Tenta executar um passo do autosave caso o intervalo tenha sido atingido e haja modificações.
    /// Retorna `Some(Ok(path))` se o snapshot foi criado com sucesso.
    ///
    /// Serializes from the borrowed project (no extra clone of live editor
    /// state beyond encode_zip). Callers that already hold an immutable
    /// revisioned snapshot should pass that instead of the live document.
    pub fn tick(
        &mut self,
        current_time_secs: u64,
        is_dirty: bool,
        project: &Project,
        project_path: Option<&Path>,
    ) -> Option<Result<PathBuf, ProjectError>> {
        if !self.config.enabled {
            return None;
        }

        if self.config.only_when_dirty && !is_dirty {
            return None;
        }

        if current_time_secs
            < self
                .last_autosave_time
                .saturating_add(self.config.interval_secs)
        {
            return None;
        }

        self.last_autosave_time = current_time_secs;
        self.seq = self.seq.wrapping_add(1);

        let result = Self::perform_snapshot(
            project,
            project_path,
            current_time_secs,
            self.seq,
            self.config.keep_n,
        );

        Some(result)
    }

    /// Captures an immutable snapshot suitable for a worker thread.
    /// At most one outstanding autosave should run per document (caller coalesces).
    pub fn capture_snapshot(project: &Project) -> Project {
        project.clone()
    }

    /// Executa gravação atômica do snapshot de recuperação e poda os mais antigos.
    pub fn perform_snapshot(
        project: &Project,
        project_path: Option<&Path>,
        timestamp: u64,
        seq: u64,
        keep_n: usize,
    ) -> Result<PathBuf, ProjectError> {
        let dir = Self::autosave_dir(project_path);
        fs::create_dir_all(&dir).map_err(|e| ProjectError::Io(e.to_string()))?;

        let filename = format!("autosave-{timestamp:010}-{seq:04}.petunia");
        let target_path = dir.join(filename);

        // Grava de forma atômica
        format::save_atomic(project, &target_path)?;

        // Aplica política de retenção (keep_n)
        let _ = Self::prune_retention(&dir, keep_n);

        Ok(target_path)
    }

    /// Remove snapshots mais antigos que excederem o limite `keep_n`.
    pub fn prune_retention(dir: &Path, keep_n: usize) -> Result<usize, std::io::Error> {
        if keep_n == 0 {
            return Ok(0);
        }

        let mut entries = Vec::new();
        if !dir.exists() {
            return Ok(0);
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file()
                && let Some(name) = path.file_name().and_then(|n| n.to_str())
                && name.starts_with("autosave-")
                && name.ends_with(".petunia")
            {
                entries.push(path);
            }
        }

        // Ordena por nome (que inclui timestamp zero-padded e sequence)
        entries.sort();

        let mut removed = 0;
        if entries.len() > keep_n {
            let excess = entries.len() - keep_n;
            for path in entries.iter().take(excess) {
                if fs::remove_file(path).is_ok() {
                    removed += 1;
                }
            }
        }

        Ok(removed)
    }

    /// Detecta se a sessão anterior encerrou de forma inesperada (unclean shutdown)
    /// e se existem snapshots válidos para recuperação.
    pub fn detect_recovery(project_path: Option<&Path>) -> Option<RecoveryInfo> {
        let lock_path = Self::session_lock_path(project_path);
        if !lock_path.exists() {
            // Encerramento limpo anterior ou primeiro início
            return None;
        }

        // Lê os metadados do lock
        let lock_info: Option<SessionLockInfo> = fs::read(&lock_path)
            .ok()
            .and_then(|b| postcard::from_bytes(&b).ok());

        let project_name = lock_info
            .as_ref()
            .map(|i| i.project_name.clone())
            .unwrap_or_else(|| "Untitled".to_string());

        let dir = Self::autosave_dir(project_path);
        if !dir.exists() {
            return None;
        }

        // Procura todos os snapshots de autosave
        let mut snapshots = Vec::new();
        if let Ok(rd) = fs::read_dir(&dir) {
            for entry in rd.flatten() {
                let path = entry.path();
                if path.is_file()
                    && let Some(name) = path.file_name().and_then(|n| n.to_str())
                    && name.starts_with("autosave-")
                    && name.ends_with(".petunia")
                {
                    snapshots.push(path);
                }
            }
        }

        if snapshots.is_empty() {
            return None;
        }

        snapshots.sort();
        let newest_snapshot = snapshots.pop()?;

        // Determina se o snapshot é mais recente que o arquivo principal (quando existir)
        let mut is_newer = true;
        let mut main_path_buf = None;

        if let Some(p) = project_path {
            main_path_buf = Some(p.to_path_buf());
            if p.exists()
                && let (Ok(snap_meta), Ok(main_meta)) =
                    (fs::metadata(&newest_snapshot), fs::metadata(p))
                && let (Ok(snap_time), Ok(main_time)) = (snap_meta.modified(), main_meta.modified())
            {
                is_newer = snap_time > main_time;
            }
        }

        let snap_time = fs::metadata(&newest_snapshot)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Some(RecoveryInfo {
            snapshot_path: newest_snapshot,
            project_name,
            snapshot_time: snap_time,
            main_project_path: main_path_buf,
            is_newer_than_main: is_newer,
        })
    }

    /// Descarta todos os snapshots de recuperação e remove o marcador de sessão.
    pub fn discard_recovery(project_path: Option<&Path>) -> Result<(), std::io::Error> {
        let dir = Self::autosave_dir(project_path);
        if dir.exists() {
            let _ = fs::remove_dir_all(&dir);
        }
        Self::remove_session_lock(project_path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use petunia_mesh::Mesh;

    #[test]
    fn test_autosave_tick_only_triggers_when_dirty() {
        let mut service = AutosaveService::new(AutosaveConfig {
            enabled: true,
            interval_secs: 10,
            keep_n: 3,
            only_when_dirty: true,
        });

        let p = Project::new();
        let dir = std::env::temp_dir().join(format!("petunia_as_test_{}", uuid::Uuid::new_v4()));
        let proj_path = dir.join("proj.petunia");

        // Caso 1: Não dirty -> não deve disparar
        let res = service.tick(100, false, &p, Some(&proj_path));
        assert!(res.is_none());

        // Caso 2: Dirty -> dispara snapshot
        let res = service.tick(100, true, &p, Some(&proj_path));
        assert!(res.is_some());
        let snap_path = res.unwrap().unwrap();
        assert!(snap_path.exists());

        // Caso 3: Mesmo tempo ou intervalo não atingido -> não dispara
        let res = service.tick(105, true, &p, Some(&proj_path));
        assert!(res.is_none());

        // Caso 4: Intervalo atingido -> dispara novo snapshot
        let res = service.tick(111, true, &p, Some(&proj_path));
        assert!(res.is_some());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_autosave_retention_pruning() {
        let dir =
            std::env::temp_dir().join(format!("petunia_retention_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();

        let mut p = Project::new();
        p.add("TestMesh", Mesh::cube(1.0));

        let proj_path = dir.join("test.petunia");

        // Cria 7 snapshots com limite keep_n = 3
        for i in 1..=7 {
            let snap =
                AutosaveService::perform_snapshot(&p, Some(&proj_path), 1000 + i, i, 3).unwrap();
            assert!(snap.exists());
        }

        // Verifica que restaram exatamente 3 arquivos
        let as_dir = AutosaveService::autosave_dir(Some(&proj_path));
        let remaining: Vec<_> = fs::read_dir(&as_dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        assert_eq!(
            remaining.len(),
            3,
            "Retenção deve manter exatamente keep_n=3 arquivos"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_session_lock_and_recovery_detection() {
        let dir =
            std::env::temp_dir().join(format!("petunia_recovery_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let proj_path = dir.join("adventure.petunia");

        let mut p = Project::new();
        p.name = "Adventure".to_string();

        // 1. Início de sessão: cria lock
        AutosaveService::create_session_lock(Some(&proj_path), &p.name, 5000).unwrap();

        // 2. Autosave grava snapshot
        let snap_path =
            AutosaveService::perform_snapshot(&p, Some(&proj_path), 5010, 1, 5).unwrap();
        assert!(snap_path.exists());

        // 3. Simula crash (lock permanece no disco sem clean shutdown)
        let rec = AutosaveService::detect_recovery(Some(&proj_path));
        assert!(rec.is_some(), "Crash marker deve detectar recuperação");
        let info = rec.unwrap();
        assert_eq!(info.project_name, "Adventure");
        assert_eq!(info.snapshot_path, snap_path);

        // 4. Descarte da recuperação limpa snapshots e lock
        AutosaveService::discard_recovery(Some(&proj_path)).unwrap();
        assert!(!AutosaveService::session_lock_path(Some(&proj_path)).exists());
        assert!(!snap_path.exists());

        let _ = fs::remove_dir_all(&dir);
    }
}
