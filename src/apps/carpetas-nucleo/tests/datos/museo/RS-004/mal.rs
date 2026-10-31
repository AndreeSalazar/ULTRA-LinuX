pub fn nada() -> bool {
    // SAFETY: ninguna; este es el patron malo.
    unsafe { core::mem::uninitialized() }
}
