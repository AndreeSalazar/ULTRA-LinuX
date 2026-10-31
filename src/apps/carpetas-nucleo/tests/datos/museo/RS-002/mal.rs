pub fn bits(f: f32) -> u32 {
    // SAFETY: f32 y u32 miden lo mismo.
    unsafe { core::mem::transmute::<f32, u32>(f) }
}
