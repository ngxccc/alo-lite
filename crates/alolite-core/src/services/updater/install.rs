//! Transactional release installation and atomic staging.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Explicit destination selected by the platform adapter, not remote metadata.
pub enum InstallTarget {
    /// Replace a portable executable and preserve its permissions.
    Portable(PathBuf),
    /// Save an installer for the platform launcher.
    Installer(PathBuf),
}

static NEXT_STAGE: AtomicU64 = AtomicU64::new(0);

fn staging_file(destination: &Path) -> Result<(PathBuf, File), String> {
    let parent = destination
        .parent()
        .ok_or("Đường dẫn cập nhật thiếu thư mục")?;
    for _ in 0..100 {
        let id = NEXT_STAGE.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!(".ctdo-update-{}-{id}", std::process::id()));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("Tạo file cập nhật tạm thất bại: {error}")),
        }
    }
    Err("Không tạo được file cập nhật tạm riêng biệt".into())
}

fn remove_staging(path: &Path) {
    // Cleanup must not hide the primary download/install failure.
    let _ = fs::remove_file(path);
}

/// Portable installation staged without replacing or launching the running executable.
/// Dropping an uncommitted transaction preserves the original executable.
pub struct DeferredPortableInstall {
    prepared: PreparedInstall,
}

impl DeferredPortableInstall {
    /// Replace the executable and launch only after the caller has joined its worker.
    ///
    /// # Errors
    /// Returns replacement, launch or explicit rollback errors.
    pub fn launch_and_finish(
        self,
        launch: impl FnOnce(&Path) -> Result<(), String>,
    ) -> Result<(), String> {
        self.prepared.finish(launch)
    }

    /// Discard the staged update and preserve the original executable.
    ///
    /// # Errors
    /// Returns a rollback error if replacement happened before cancellation.
    pub fn abort(mut self) -> Result<(), String> {
        self.prepared.rollback()
    }
}

/// Stage validated portable bytes without replacing or launching any executable.
///
/// # Errors
/// Returns filesystem errors while preparing private sibling files.
pub fn stage_portable_release(
    bytes: &[u8],
    destination: &Path,
) -> Result<DeferredPortableInstall, String> {
    Ok(DeferredPortableInstall {
        prepared: prepare_install(bytes, destination, true)?,
    })
}

struct PreparedInstall {
    destination: PathBuf,
    stage: PathBuf,
    backup: Option<PathBuf>,
    installed: bool,
    committed: bool,
}

impl PreparedInstall {
    fn finish(mut self, launch: impl FnOnce(&Path) -> Result<(), String>) -> Result<(), String> {
        fs::rename(&self.stage, &self.destination)
            .map_err(|error| format!("Thay thế gói cập nhật thất bại: {error}"))?;
        self.installed = true;
        if let Err(error) = launch(&self.destination) {
            self.rollback().map_err(|rollback| {
                format!("Khởi chạy thất bại: {error}; hoàn tác thất bại: {rollback}")
            })?;
            return Err(format!("Khởi chạy thất bại, đã hoàn tác cập nhật: {error}"));
        }
        self.committed = true;
        Ok(())
    }

    fn rollback(&mut self) -> Result<(), String> {
        if self.installed && !self.committed {
            match &self.backup {
                Some(backup) => fs::rename(backup, &self.destination),
                None => fs::remove_file(&self.destination),
            }
            .map_err(|error| format!("Khôi phục gói cập nhật cũ thất bại: {error}"))?;
            self.installed = false;
        }
        Ok(())
    }
}

impl Drop for PreparedInstall {
    fn drop(&mut self) {
        // A retained backup is the recovery source if rollback itself fails.
        if self.rollback().is_err() {
            return;
        }
        remove_staging(&self.stage);
        if let Some(backup) = &self.backup {
            remove_staging(backup);
        }
    }
}

fn prepare_install(
    bytes: &[u8],
    destination: &Path,
    portable: bool,
) -> Result<PreparedInstall, String> {
    let metadata = match fs::symlink_metadata(destination) {
        Ok(metadata) if metadata.file_type().is_file() => Some(metadata),
        Ok(_) => return Err("Đích cập nhật không phải file thông thường".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && !portable => None,
        Err(error) => return Err(format!("Đọc đích cập nhật thất bại: {error}")),
    };
    let (stage, mut file) = staging_file(destination)?;
    let prepared = (|| {
        file.write_all(bytes)?;
        if portable {
            if let Some(metadata) = &metadata {
                file.set_permissions(metadata.permissions())?;
            }
        }
        file.sync_all()
    })();
    drop(file);
    if let Err(error) = prepared {
        remove_staging(&stage);
        return Err(format!("Ghi file cập nhật thất bại: {error}"));
    }
    let backup = if metadata.is_some() {
        let (path, file) = match staging_file(destination) {
            Ok(value) => value,
            Err(error) => {
                remove_staging(&stage);
                return Err(error);
            }
        };
        drop(file);
        // A private sibling copy keeps the original destination present until atomic replacement.
        if let Err(error) = fs::copy(destination, &path) {
            remove_staging(&stage);
            remove_staging(&path);
            return Err(format!("Sao lưu bản cập nhật cũ thất bại: {error}"));
        }
        Some(path)
    } else {
        None
    };
    Ok(PreparedInstall {
        destination: destination.to_path_buf(),
        stage,
        backup,
        installed: false,
        committed: false,
    })
}

/// Atomically install already-validated bytes and invoke the injected launcher.
/// Existing bytes are restored if replacement or launch fails. A rollback failure
/// is returned explicitly and the backup is retained for recovery.
///
/// # Errors
/// Returns filesystem or launcher errors; never falls back to another destination.
pub fn install_release(
    bytes: &[u8],
    target: &InstallTarget,
    launch: impl FnOnce(&Path) -> Result<(), String>,
) -> Result<(), String> {
    let (destination, portable) = match target {
        InstallTarget::Portable(path) => (path, true),
        InstallTarget::Installer(path) => (path, false),
    };
    prepare_install(bytes, destination, portable)?.finish(launch)
}
