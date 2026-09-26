pub mod app;

pub mod cmd;
pub mod config;
pub mod decorations;
pub mod editor;
pub mod icons;
pub mod pages;
pub mod panes;
pub mod shared;

#[cfg(target_arch = "wasm32")]
mod web;

use crate::app::App;

/// Initialises the tracing subscriber for the current environment.
///
/// On web, it uses `tracing_wasm` and on native, `tracing_tree` is used.
pub fn setup_tracing() {
    #[cfg(target_arch = "wasm32")]
    {
        tracing_wasm::set_as_global_default();
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        use tracing_subscriber::Layer;
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::util::SubscriberInitExt;

        use crate::shared::mem_logger::GlobalMemLogLayer;

        let target_filter =
            tracing_subscriber::filter::filter_fn(|meta| meta.target().contains("mktools"));

        let mem_layer =
            GlobalMemLogLayer::new().with_filter(tracing_subscriber::EnvFilter::new("debug"));

        #[cfg(debug_assertions)]
        {
            let console_layer = tracing_tree::HierarchicalLayer::new(2)
                .with_indent_lines(true)
                .with_targets(true)
                .with_bracketed_fields(true);

            tracing_subscriber::registry()
                .with(target_filter)
                .with(mem_layer)
                .with(console_layer)
                .init();
        }

        #[cfg(not(debug_assertions))]
        {
            tracing_subscriber::registry()
                .with(target_filter)
                .with(mem_layer)
                .init();
        }
    }

    tracing::debug!("Logging initialized");
}

fn window_builder_hook(builder: egui::ViewportBuilder) -> egui::ViewportBuilder {
    builder
        .with_title("Slipstream")
        .with_inner_size(egui::Vec2::new(600.0, 200.0))
        .with_decorations(false)
        .with_resizable(false)
}

/// Starts the entire application.
///
/// This function sets up logging and the [`eframe`] instance before launching the application.
pub fn run() -> SlipstreamResult<()> {
    setup_tracing();

    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::sync::Arc;

        use eframe::egui_wgpu::SurfaceErrorAction;

        let wgpu_setup = eframe::egui_wgpu::WgpuSetupCreateNew {
            instance_descriptor: wgpu::InstanceDescriptor {
                backends: wgpu::Backends::PRIMARY,
                backend_options: wgpu::BackendOptions::default(),
                display: None,
                flags: wgpu::InstanceFlags::empty(),
                memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
            },
            device_descriptor: Arc::new(|adapter| {
                let adapter_info = adapter.get_info();

                tracing::info!("Using adapter {}", adapter_info.name);

                wgpu::DeviceDescriptor {
                    label: Some("device"),
                    experimental_features: wgpu::ExperimentalFeatures::disabled(),
                    memory_hints: wgpu::MemoryHints::Performance,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::defaults(),
                    trace: wgpu::Trace::Off,
                }
            }),
            native_adapter_selector: None,
            display_handle: None,
            power_preference: wgpu::PowerPreference::default(),
        };

        let wgpu_options = eframe::WgpuConfiguration {
            on_surface_status: Arc::new(|status| match status {
                wgpu::CurrentSurfaceTexture::Outdated => {
                    tracing::debug!("Dropped frame with error: {status:?}");
                    SurfaceErrorAction::Reconfigure
                }
                wgpu::CurrentSurfaceTexture::Lost => {
                    tracing::debug!("Dropped frame with error: {status:?}");
                    SurfaceErrorAction::RecreateSurface
                }
                wgpu::CurrentSurfaceTexture::Occluded => SurfaceErrorAction::SkipFrame,
                _ => {
                    tracing::warn!("Dropped frame with error: {status:?}");
                    SurfaceErrorAction::SkipFrame
                }
            }),
            surface: eframe::SurfaceConfig::HIGH_THROUGHPUT,
            wgpu_setup: eframe::egui_wgpu::WgpuSetup::CreateNew(wgpu_setup),
        };

        let native_options = eframe::NativeOptions {
            centered: true,
            dithering: true,
            renderer: eframe::Renderer::Wgpu,
            window_builder: Some(Box::new(window_builder_hook)),
            wgpu_options,
            ..Default::default()
        };

        eframe::run_native(
            "Slipstream",
            native_options,
            Box::new(|cc| Ok(Box::new(App::new(cc)))),
        )?;
    }

    Ok(())
}
