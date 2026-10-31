use core::mem::MaybeUninit;
pub fn cero() -> u32 {
    let mut x = MaybeUninit::<u32>::uninit();
    x.write(0);
    // SAFETY: se acaba de escribir.
    unsafe { x.assume_init() }
}
