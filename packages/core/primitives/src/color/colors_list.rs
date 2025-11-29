// Lightweight color lists copied from core's color implementation (trimmed for primitives)
// Add a small public list of named colors if necessary; for now keep minimal.

pub fn basic_colors() -> Vec<(u32, &'static str)> {
  vec![
    (0x000000, "black"),
    (0xFFFFFF, "white"),
    (0xFF0000, "red"),
    (0x00FF00, "green"),
    (0x0000FF, "blue"),
  ]
}
