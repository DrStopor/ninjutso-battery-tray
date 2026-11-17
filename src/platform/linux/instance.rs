//! Kernel-backed per-user lock; keep the inode when unlocking to avoid races.
use std::fs::{File, OpenOptions, TryLockError};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

pub(crate) struct SingleInstanceGuard {
    _file: File,
}

impl SingleInstanceGuard {
    pub(crate) fn acquire() -> Result<Option<Self>, String> {
        let runtime = std::env::var_os("XDG_RUNTIME_DIR")
            .map(std::path::PathBuf::from)
            .filter(|path| path.is_absolute());
        let directory = match runtime {
            Some(path) => path,
            None => super::settings::config_directory()?,
        };
        super::settings::private_directory(&directory)?;
        Self::at(&directory.join("ninjutso-battery-tray.lock"))
    }

    fn at(path: &Path) -> Result<Option<Self>, String> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)
            .map_err(|e| format!("Could not open instance lock: {e}"))?;
        match file.try_lock() {
            Ok(()) => Ok(Some(Self { _file: file })),
            Err(TryLockError::WouldBlock) => Ok(None),
            Err(error) => Err(format!("Could not lock application instance: {error}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lock_is_exclusive_and_is_released_without_removing_the_inode() {
        let path = std::env::temp_dir().join(format!("ninjutso-lock-test-{}", std::process::id()));
        let first = SingleInstanceGuard::at(&path).unwrap().unwrap();
        assert!(SingleInstanceGuard::at(&path).unwrap().is_none());
        drop(first);
        assert!(path.exists());
        assert!(SingleInstanceGuard::at(&path).unwrap().is_some());
        std::fs::remove_file(path).unwrap();
    }
}
