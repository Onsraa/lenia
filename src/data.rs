use bevy::prelude::*;
use bevy::render::render_resource::ShaderType;
use serde::{Deserialize, Serialize};

#[derive(Resource, ShaderType, Default, Debug, Clone, Copy)]
pub struct GpuSimulationParams {
    pub time: f32,
    pub delta_time: f32,
    pub growth_rate: f32,
    pub kernel_radius: f32,
    pub grid_size: UVec2,
    pub pause: u32,
}

#[derive(Resource, Serialize, Deserialize, Debug, Clone)]
pub struct SimulationSettings {
    pub grid_size: UVec2,
    pub growth_rate: f32,
    pub kernel_radius: f32,
    pub time_scale: f32,
    pub pause: bool,
}

impl Default for SimulationSettings {
    fn default() -> Self {
        Self {
            grid_size: UVec2::new(256, 256),
            growth_rate: 0.1,
            kernel_radius: 10.0,
            time_scale: 1.0,
            pause: false,
        }
    }
}