#[derive(Copy, Clone, PartialEq)]
pub enum RenderMode {
    Naive,
    ForwardPlus,
}

pub struct Settings {
    pub light_count: u32,
    pub render_mode: RenderMode,
}
