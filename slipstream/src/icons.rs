use slipstream_ir::node::node::IrNodeType;

egui_phosphor::subset! {
    pub mod icons {
        use regular::{
            FOLDER, FOLDER_OPEN, FOLDER_MINUS, FOLDER_PLUS, X, INFO, MINUS, SQUARE, FOLDER_DASHED, BONE,
            QUESTION_MARK, FILE, CUBE, POLYGON, ARROW_ELBOW_RIGHT, MOON, GEAR_FINE, GITHUB_LOGO, SUN, POWER,
            FILE_CODE, PAINT_BRUSH_HOUSEHOLD, BOUNDING_BOX, LINK, PALETTE, GRAPHICS_CARD, PERSON
        };
        use fill::{FOLDER, BONE};
    }
}

/// Loads the given in regular font.
///
/// Make sure the icon you want is added to the list of imported icons.
#[macro_export]
macro_rules! reg_icon {
    ($icon:ident) => {
        // $crate::icons::icons::regular::rich($crate::icons::icons::regular::$icon)
        egui::RichText::new($crate::icons::icons::regular::$icon)
    };
}

/// Loads the given in filled font.
///
/// Make sure the icon you want is added to the list of imported icons.
#[macro_export]
macro_rules! fill_icon {
    ($icon:ident) => {
        $crate::icons::icons::fill::rich($crate::icons::icons::fill::$icon)
    };
}

/// Extends [`IrNodeType`], providing UI specific utilities to node types.
pub trait NodeIconsExt {
    /// The icon to display when this node is open.
    fn open_icon(&self) -> egui::RichText;
    /// The icon to display when this node is closed.
    fn closed_icon(&self) -> egui::RichText;
}

impl NodeIconsExt for IrNodeType {
    fn open_icon(&self) -> egui::RichText {
        match self {
            Self::ArcDirectory { empty: false } | Self::BrresFile | Self::Nw4rDirectory => {
                reg_icon!(FOLDER_OPEN)
            }
            // Just reuse the closed icon for everything else.
            _ => self.closed_icon(),
        }
    }

    fn closed_icon(&self) -> egui::RichText {
        match self {
            Self::ArcDirectory { empty: false } | Self::Nw4rDirectory => reg_icon!(FOLDER),
            Self::ArcDirectory { empty: true } => reg_icon!(FOLDER_DASHED),
            Self::BrresFile => reg_icon!(FOLDER),
            Self::Mdl0Root => reg_icon!(PERSON),
            Self::Definitions => reg_icon!(FILE_CODE),
            Self::Bone { end: false } => reg_icon!(BONE),
            Self::Bone { end: true } => fill_icon!(BONE),
            Self::VertexBuffer => reg_icon!(POLYGON),
            Self::NormalBuffer => reg_icon!(ARROW_ELBOW_RIGHT),
            Self::ColorBuffer => reg_icon!(PAINT_BRUSH_HOUSEHOLD),
            Self::UvBuffer => reg_icon!(BOUNDING_BOX),
            Self::Material => reg_icon!(PALETTE),
            Self::Tevs => reg_icon!(GRAPHICS_CARD),
            Self::Polygon => reg_icon!(CUBE),
            Self::TextureLinks => reg_icon!(LINK),
            Self::PaletteLinks => reg_icon!(LINK),
            Self::Unknown => reg_icon!(FILE),
        }
    }
}
