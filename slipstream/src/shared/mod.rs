pub mod camera;
pub mod inspect;
pub mod mem_logger;
pub mod vertex;
pub mod wgsl_include;
pub mod widgets;

use std::sync::Arc;

use eframe::egui_wgpu;
use egui::mutex::RwLock;

#[derive(Clone)]
pub struct GraphicsState {
    pub instance: wgpu::Instance,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub renderer: Arc<RwLock<egui_wgpu::Renderer>>,
}
