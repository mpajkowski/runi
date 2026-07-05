use std::{
    env,
    fs::{self, File},
    path::PathBuf,
};

pub struct Lock(Option<LockPriv>);

struct LockPriv {
    file: File,
    path: PathBuf,
}

impl Drop for Lock {
    fn drop(&mut self) {
        let Some(LockPriv { file, path }) = self.0.take() else {
            return;
        };

        let _ = file.unlock();
        let _ = std::fs::remove_file(path);
    }
}

impl Lock {
    pub fn obtain() -> Option<Self> {
        let dir = env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_owned());
        let display = env::var("WAYLAND_DISPLAY")
            .or_else(|_| env::var("DISPLAY").map(|c| c.replace(':', "x")))
            .unwrap_or_default();

        let path = PathBuf::from(dir).join(format!("runi-{display}.lock"));

        log::debug!("flock file: {}", path.display());

        let file = match File::create(&path) {
            Ok(f) => f,
            Err(err) => {
                log::warn!("failed to create a lock file {}: {err}", path.display());
                return Some(Self::unlocked());
            }
        };

        if let Err(err) = file.try_lock() {
            match err {
                fs::TryLockError::Error(err) => {
                    log::warn!("failed to lock a file {}: {err}", path.display());
                    return Some(Self::unlocked());
                }
                fs::TryLockError::WouldBlock => return None,
            }
        }

        Some(Self(Some(LockPriv { file, path })))
    }

    pub const fn unlocked() -> Self {
        Self(None)
    }
}
