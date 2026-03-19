pub fn height_from_width(width: u32, original_width: u32, original_height: u32) -> u32 {
    ((width as u64 * original_height as u64) / original_width.max(1) as u64).max(1) as u32
}

pub fn width_from_height(height: u32, original_width: u32, original_height: u32) -> u32 {
    ((height as u64 * original_width as u64) / original_height.max(1) as u64).max(1) as u32
}
