pub fn ver(p: *const u8, n: usize) -> &'static [u8] {
    // SAFETY: el llamador lo promete (y nada lo comprueba).
    unsafe { core::slice::from_raw_parts(p, n) }
}
