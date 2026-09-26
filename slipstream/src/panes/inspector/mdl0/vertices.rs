use crate::format::mdl0::vertices::VertexBuf;
use crate::node::node::Inspectable;
use crate::panes::inspector::widgets::{draw_inspector_section_header, draw_vec_drag_values};

impl Inspectable for VertexBuf {
    fn draw_properties(&mut self, ui: &mut egui::Ui) {
        let input_field_size = egui::vec2(180.0, 20.0);

        draw_inspector_section_header("Bounding volume".to_owned(), ui);

        egui::Grid::new("vertices_inspector_grid1")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Bounding volume minimum:");
                draw_vec_drag_values(
                    input_field_size,
                    ["X:", "Y:", "Z:"],
                    &mut self.bounding_volume_min,
                    ui,
                );

                ui.end_row();

                ui.label("Bounding volume maximum:");
                draw_vec_drag_values(
                    input_field_size,
                    ["X:", "Y:", "Z:"],
                    &mut self.bounding_volume_max,
                    ui,
                );
            });

        draw_inspector_section_header("Vertices".to_owned(), ui);

        ui.label(format!("Vertex count: {}", self.vertices.len()));
        ui.label(format!("Vertex format: {:?}", self.format));
        ui.label(format!("Divisor: {}", self.divisor));
        ui.label(format!("Stride: {}", self.stride));

        let divisor = egui::DragValue::new(&mut self.divisor);
        ui.add(divisor);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
