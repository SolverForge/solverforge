use std::sync::{Mutex, MutexGuard, OnceLock};

static CWD_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub(crate) fn lock_cwd() -> MutexGuard<'static, ()> {
    CWD_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("cwd lock poisoned")
}
