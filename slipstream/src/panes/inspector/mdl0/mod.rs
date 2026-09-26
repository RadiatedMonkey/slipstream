use std::any::Any;
use egui::Ui;
use crate::format::mdl0::Model;
use crate::node::node::Inspectable;

pub mod bones;
pub mod bytecode;
pub mod colors;
pub mod materials;
pub mod normals;
pub mod pal_links;
pub mod shapes;
pub mod tevs;
pub mod tex_links;
pub mod uvs;
pub mod vertices;

impl Inspectable for Model {
    fn draw_properties(&mut self, ui: &mut Ui) {
        ui.label(format!("{self:#?}"));
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}