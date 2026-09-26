use crate::format::mdl0::bones::{BillboardSetting, BoneFlags, VirtualBone};
use crate::node::node::{Inspectable, VirtualNodeKind};
use crate::panes::inspector::widgets::{
    draw_inspector_section_header, draw_node_reference, draw_vec_drag_values,
    draw_vec_drag_values_suffixed,
};

const BILLBOARD_SETTING_DESCRIPTIONS: &[&str] = &[
    "No influence",
    "Influenced by rotation of the parent node. Z-axis is parallel to camera lens axis",
    "Influenced by rotation of the parent node. Z-axis points toward camera direction",
    "Not influenced by rotation of parent node, restricted to camera's up vector. Z-axis is parallel to camera lens axis",
    "Not influenced by rotation of parent node, restricted to camera's up vector. Z-axis points toward camera direction",
    "Influenced by rotation of parent node and rotates only around Y-axis. Z-axis is parallel to the camera lens axis",
    "Influenced by rotation of parent node and rotates only around Y-axis. Z-axis points toward camera direction",
];

impl Inspectable for BoneFlags {
    fn draw_properties(&mut self, ui: &mut egui::Ui) {
        todo!("make checkboxes work with bit struct");

        // ui.checkbox(
        //     &mut self.apply_child_scale_compensate(),
        //     "Enable child scale compensate",
        // )
        // .on_hover_text("Whether this bone compensates for a parent's non-uniform scaling, preventing child meshes from stretching unnaturally.");

        // ui.checkbox(
        //     &mut self.apply_scale_compensate(),
        //     "Enable scale compensate",
        // )
        // .on_hover_text(
        //     "Reverses parent scale on the current so local transformations remain unscaled.",
        // );
        // ui.end_row();

        // ui.checkbox(&mut self.disable_classic_scale(), "Disable classic scale")
        //     .on_hover_text("Controls the scale matrix algorithm. When set, disables traditional Maya/Wii scaling behaviour (where parent scale propagates directly down the hierarchy) in favour of standard matrix multiplication");

        // ui.checkbox(&mut self.is_billboard_child(), "Billboard child")
        //     .on_hover_text(
        //         "Indicates that this bone's parent is a billboard, i.e. always facing the camera",
        //     );
        // ui.end_row();

        // ui.checkbox(&mut self.is_display_matrix(), "Is display matrix")
        //     .on_hover_text("Set if this bone is directly tied to a draw element/mesh node and requires a matrix in the draw matrix table for GPU skinning and rendering");

        // ui.checkbox(&mut self.is_visible(), "Visible")
        //     .on_hover_text("Whether the geometry attached to this bone is visible");
        // ui.end_row();

        // ui.checkbox(
        //     &mut self.translation_isotropic(),
        //     "Enable isotropic translation",
        // )
        // .on_hover_text("Set when the translation vector is isotropic, i.e. all values are equal");

        // ui.checkbox(&mut self.rotation_isotropic(), "Enable isotropic rotation")
        //     .on_hover_text("Set when the rotation vector is isotropic, i.e. all values are equal");

        // ui.end_row();

        // ui.checkbox(&mut self.scale_isotropic(), "Enable isotropic scaling")
        //     .on_hover_text("Set when the scaling vector is isotropic, i.e. all values are equal");

        // ui.checkbox(&mut self.scale_uniform(), "Enable uniform scaling")
        //     .on_hover_text("Optimization flag set when all scaling factors are equal");
        // ui.end_row();

        // ui.checkbox(&mut self.use_identity(), "Use identity").on_hover_text("Fast-path for rendering, forces the engine to use the identity matrix for this bone (i.e. no translation, rotation or scale)");
        // ui.end_row();
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl Inspectable for VirtualBone {
    fn draw_properties(&mut self, ui: &mut egui::Ui) {
        let input_field_size = egui::vec2(180.0, 20.0);

        draw_inspector_section_header("Transformation".to_owned(), ui);

        egui::Grid::new("bone_inspector_grid1")
            .striped(true)
            .num_columns(2)
            .spacing([40.0, 8.0])
            .show(ui, |ui| {
                ui.label("Translation:");
                draw_vec_drag_values(
                    input_field_size,
                    ["X:", "Y:", "Z:"],
                    &mut self.translation_vector,
                    ui,
                );

                ui.end_row();

                ui.label("Rotation:");
                draw_vec_drag_values_suffixed(
                    input_field_size,
                    ["X:", "Y:", "Z:"],
                    " °",
                    &mut self.rotation_vector,
                    ui,
                );

                ui.end_row();

                ui.label("Scale:");
                draw_vec_drag_values(
                    input_field_size,
                    ["X:", "Y:", "Z:"],
                    &mut self.scaling_vector,
                    ui,
                );

                ui.end_row();

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
                ui.end_row();
            });

        draw_inspector_section_header("Skeleton hierarchy".to_owned(), ui);

        ui.label(format!("Parent: {:?}", self.parent));
        // ui.add(egui::DragValue::new(&mut self.parent));

        ui.separator();

        ui.label(format!("Bone index: {}", self.index));
        ui.add(egui::DragValue::new(&mut self.index));

        ui.end_row();

        ui.horizontal(|ui| {
            ui.label("Billboard reference:");
            draw_node_reference(
                egui::Id::new("billboard_bone_ref"),
                self.billboard_reference,
                ui,
            );
            // ui.add(egui::DragValue::new(&mut self.billboard_reference));

            ui.add_space(0.1 * ui.available_width());

            ui.label("Billboard setting:");
            ui.menu_button(format!("{:?}", self.billboard_setting), |ui| {
                for i in 0..BillboardSetting::len() {
                    let setting = BillboardSetting::try_from(i as u32)
                        .expect("billboard setting size is outdated");

                    if ui
                        .button(format!("{setting:?}"))
                        .on_hover_text(BILLBOARD_SETTING_DESCRIPTIONS[i])
                        .clicked()
                    {
                        self.billboard_setting = setting;
                    }
                }
            });
        });

        draw_inspector_section_header("Flags".to_owned(), ui);

        egui::Grid::new("bone_inspector_grid2")
            .num_columns(2)
            .show(ui, |ui| {
                self.flags.draw_properties(ui);
            });

        draw_inspector_section_header("User data".to_owned(), ui);

        ui.label("User data:");
        ui.add(egui::DragValue::new(&mut self.user_data_offset));
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
