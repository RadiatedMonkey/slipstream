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

pub trait NodeVisualsExt {
    /// Whether this node can be opened like a folder.
    fn is_expandable(&self) -> bool;
    /// The icon to display when this node is open.
    ///
    /// This is only used for nodes that can be expanded.
    fn open_icon(&self) -> egui::RichText;
    /// The icon to display when this node is closed.
    ///
    /// This is also the icon used for nodes that cannot be opened.
    fn closed_icon(&self) -> egui::RichText;
}

impl NodeVisualsExt for IrNodeType {
    fn is_expandable(&self) -> bool {
        match self {
            Self::ArcDirectory { empty: false } | Self::BrresDirectory => true,
            _ => false,
        }
    }

    fn open_icon(&self) -> egui::RichText {
        match self {
            Self::ArcDirectory { empty: false } | Self::BrresDirectory => reg_icon!(FOLDER_OPEN),
            _ => unimplemented!("cannot call `NodeVisualsExt::open_icon` on a non-expandable node"),
        }
    }

    fn closed_icon(&self) -> egui::RichText {
        match self {
            Self::ArcDirectory { empty: false } | Self::BrresDirectory => reg_icon!(FOLDER),
            Self::ArcDirectory { empty: true } => reg_icon!(FOLDER_DASHED),
            Self::Mdl0Root => reg_icon!(PERSON),
            Self::Bytecode => reg_icon!(FILE_CODE),
            Self::Bone { end: false } => reg_icon!(BONE),
            Self::Bone { end: true } => fill_icon!(BONE),
            Self::Vertices => reg_icon!(POLYGON),
            Self::Normals => reg_icon!(ARROW_ELBOW_RIGHT),
            Self::Colors => reg_icon!(PAINT_BRUSH_HOUSEHOLD),
            Self::Uvs => reg_icon!(BOUNDING_BOX),
            Self::Materials => reg_icon!(PALETTE),
            Self::Tevs => reg_icon!(GRAPHICS_CARD),
            Self::Shape => reg_icon!(CUBE),
            Self::TextureLinks => reg_icon!(LINK),
            Self::PaletteLinks => reg_icon!(LINK),
            Self::Unknown => reg_icon!(FILE),
        }
    }
}
