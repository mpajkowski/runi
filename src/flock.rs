use std::{
    env,
    fs::{self, File},
    os::unix::fs::OpenOptionsExt,
    path::PathBuf,
};

use nix::{
    fcntl::{Flock, FlockArg},
    libc::O_CLOEXEC,
};

pub struct Lock(Option<Flock<File>>, Option<PathBuf>);

impl Drop for Lock {
    fn drop(&mut self) {
        let Some(f) = self.0.take() else {
            return;
        };

        let Ok(f) = f.unlock() else {
            return;
        };

        drop(f);

        if let Some(path) = &self.1 {
            let _ = std::fs::remove_file(path);
        }
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

        let f = match fs::OpenOptions::new()
            .create(true)
            .write(true)
            .custom_flags(O_CLOEXEC)
            .open(&path)
        {
            Ok(f) => f,
            Err(err) => {
                log::warn!("failed to open lock file {}: {err}", path.display());
                return Some(Self(None, None));
            }
        };

        let flock = nix::fcntl::Flock::lock(f, FlockArg::LockExclusiveNonblock).ok()?;

        Some(Self(Some(flock), Some(path)))
    }
}
