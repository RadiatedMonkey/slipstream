use crate::node::refs::VirtualNodeId;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DraggableNodeKind {
    Directory,
    Bone,
}

#[derive(Debug)]
pub struct DraggableNodePayload {
    pub id: VirtualNodeId,
    pub kind: DraggableNodeKind,
}

pub fn draw_node_reference(id: egui::Id, node: Option<VirtualNodeId>, ui: &mut egui::Ui) {
    let frame = egui::Frame::default().inner_margin(4.0);

    let (_response, payload) =
        ui.dnd_drop_zone::<VirtualNodeId, Option<VirtualNodeId>>(frame, |ui| {
            if let Some(node) = node {
                let response = ui
                    .dnd_drag_source(id, node, |ui| {
                        ui.label(format!("label {node:?}"));
                    })
                    .response;

                if let Some(hovered) = response.dnd_hover_payload::<VirtualNodeId>() {
                    tracing::trace!("Hovering {:?}", *hovered);
                }

                if let Some(payload) = response.dnd_release_payload::<VirtualNodeId>() {
                    tracing::trace!("Dropped {:?}", *payload);
                }
            } else {
                ui.label("None");
            }

            None
        });

    if let Some(payload) = payload {
        tracing::trace!("Dropped outer: {}", *payload);
    }
}

pub fn draw_vec_drag_values_suffixed<T: egui::emath::Numeric, const N: usize>(
    mut input_field_size: egui::Vec2,
    labels: [&str; N],
    suffix: &str,
    values: &mut [T; N],
    ui: &mut egui::Ui,
) {
    input_field_size.x /= N as f32;

    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        for i in (0..N).rev() {
            let drag_value = egui::DragValue::new(&mut values[i]).suffix(suffix);
            ui.add_sized(input_field_size, drag_value);
            if labels[i].is_empty() {
                let label_width = ui
                    .painter()
                    .layout_no_wrap(
                        "X:".to_owned(),
                        egui::FontId::default(),
                        egui::Color32::TRANSPARENT,
                    )
                    .rect
                    .width();
                ui.allocate_space(egui::vec2(label_width, input_field_size.y));
            } else {
                ui.label(labels[i]);
            }
        }
    });
}

pub fn draw_vec_drag_values<T: egui::emath::Numeric, const N: usize>(
    mut input_field_size: egui::Vec2,
    labels: [&str; N],
    values: &mut [T; N],
    ui: &mut egui::Ui,
) {
    input_field_size.x /= N as f32;

    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        for i in (0..N).rev() {
            let drag_value = egui::DragValue::new(&mut values[i]);
            ui.add_sized(input_field_size, drag_value);
            if labels[i].is_empty() {
                let label_width = ui
                    .painter()
                    .layout_no_wrap(
                        "X:".to_owned(),
                        egui::FontId::default(),
                        egui::Color32::TRANSPARENT,
                    )
                    .rect
                    .width();
                ui.allocate_space(egui::vec2(label_width, input_field_size.y));
            } else {
                ui.label(labels[i]);
            }
        }
    });
}

pub fn draw_inspector_section_header(name: String, ui: &mut egui::Ui) {
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        let total_width = ui.available_width();
        let text_galley = ui.painter().layout_no_wrap(
            name.clone(),
            egui::FontId::default(),
            ui.visuals().text_color(),
        );
        let text_width = text_galley.rect.width();

        let item_spacing = ui.spacing().item_spacing.x;
        let num_gaps = 2.0;

        let line_width = ((total_width - text_width - item_spacing * num_gaps) / 2.0).max(0.0);
        let separator1_size = egui::vec2(line_width, 1.0);

        ui.add_sized(separator1_size, egui::Separator::default().horizontal());
        ui.label(name);
        ui.add_sized(separator1_size, egui::Separator::default().horizontal());
    });
    ui.add_space(10.0);
}
