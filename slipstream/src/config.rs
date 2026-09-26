use std::time::Duration;

pub const FILE_INDENTATION_SIZE: f32 = 8.0;
pub const LAUNCH_DELAY: Duration = Duration::from_millis(500);
pub const DEFAULT_SIZE: egui::Vec2 = egui::Vec2::new(800.0, 600.0);
pub const APP_TITLE: &str = "Mario Kart Wii Editor";

/// Configures the egui style to use for the dark theme.
pub fn configure_dark_style() -> egui::Style {
    let style = egui::Theme::Dark.default_style();

    egui::Style {
        visuals: egui::Visuals {
            interact_cursor: Some(egui::CursorIcon::PointingHand),
            window_fill: egui::Color32::from_gray(30),
            widgets: egui::style::Widgets {
                active: egui::style::WidgetVisuals {
                    corner_radius: egui::CornerRadius::ZERO,
                    ..style.visuals.widgets.active
                },
                inactive: egui::style::WidgetVisuals {
                    corner_radius: egui::CornerRadius::ZERO,
                    ..style.visuals.widgets.inactive
                },
                hovered: egui::style::WidgetVisuals {
                    corner_radius: egui::CornerRadius::ZERO,
                    ..style.visuals.widgets.hovered
                },
                noninteractive: egui::style::WidgetVisuals {
                    corner_radius: egui::CornerRadius::ZERO,
                    ..style.visuals.widgets.noninteractive
                },
                open: egui::style::WidgetVisuals {
                    corner_radius: egui::CornerRadius::ZERO,
                    ..style.visuals.widgets.open
                },
                ..Default::default()
            },
            panel_fill: egui::Color32::from_gray(40),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Configures the egui style to use for the light theme.
pub fn configure_light_style() -> egui::Style {
    let style = egui::Theme::Light.default_style();

    egui::Style {
        visuals: egui::Visuals {
            interact_cursor: Some(egui::CursorIcon::PointingHand),
            panel_fill: egui::Color32::WHITE,
            widgets: egui::style::Widgets {
                active: egui::style::WidgetVisuals {
                    corner_radius: egui::CornerRadius::ZERO,
                    ..style.visuals.widgets.active
                },
                inactive: egui::style::WidgetVisuals {
                    corner_radius: egui::CornerRadius::ZERO,
                    ..style.visuals.widgets.inactive
                },
                hovered: egui::style::WidgetVisuals {
                    corner_radius: egui::CornerRadius::ZERO,
                    ..style.visuals.widgets.hovered
                },
                noninteractive: egui::style::WidgetVisuals {
                    corner_radius: egui::CornerRadius::ZERO,
                    ..style.visuals.widgets.noninteractive
                },
                open: egui::style::WidgetVisuals {
                    corner_radius: egui::CornerRadius::ZERO,
                    ..style.visuals.widgets.open
                },
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    }
}
