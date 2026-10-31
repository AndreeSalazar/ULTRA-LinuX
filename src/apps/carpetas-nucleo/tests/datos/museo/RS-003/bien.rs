use core::sync::atomic::{AtomicU32, Ordering};
static CUENTA: AtomicU32 = AtomicU32::new(0);
pub fn sube() {
    CUENTA.fetch_add(1, Ordering::Relaxed);
}
