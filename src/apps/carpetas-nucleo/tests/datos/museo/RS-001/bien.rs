pub fn leer(r: &u8) -> u8 {
    let p: *const u8 = r;
    // SAFETY: p viene de una referencia viva y alineada.
    unsafe { *p }
}
