use crate::persistent_state::PersistentState;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, Extent3d, Texture,
    TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureViewDescriptor,
};

#[derive(Debug)]
pub struct BlitState {
    pub render_texture: Texture,
    pub blit_bind_group: BindGroup,
}

impl BlitState {
    pub const RENDER_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;

    pub fn init(persistent: &PersistentState) -> Self {
        let PersistentState {
            device,
            render_texture_sampler,
            blit_bind_group_layout,
            ..
        } = persistent;
        let render_texture = {
            let (width, height) = persistent.render_texture_size();
            device.create_texture(&TextureDescriptor {
                label: Some("render_texture"),
                dimension: TextureDimension::D2,
                size: Extent3d {
                    width,
                    height,
                    ..Default::default()
                },
                mip_level_count: 1,
                sample_count: 1,
                format: Self::RENDER_TEXTURE_FORMAT,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
        };
        let render_texture_view = render_texture.create_view(&TextureViewDescriptor::default());
        let blit_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("blit_bind_group"),
            layout: blit_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&render_texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(render_texture_sampler),
                },
            ],
        });
        Self {
            render_texture,
            blit_bind_group,
        }
    }
}
