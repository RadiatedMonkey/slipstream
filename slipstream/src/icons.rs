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
