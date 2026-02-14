#[derive(Debug)]
pub struct RenderTextureConfig {
    factor: u32,
}

impl RenderTextureConfig {
    pub fn render_texture_size(&self) -> (u32, u32) {
        (160 * self.factor, 90 * self.factor)
    }

    pub fn update_render_texture_size(&mut self, delta: i32) {
        self.factor = std::cmp::max(1, self.factor.saturating_add_signed(delta));
    }
}

impl Default for RenderTextureConfig {
    fn default() -> Self {
        return Self {
            factor: 12, // 1920x1080
        };
    }
}
