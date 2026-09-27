use slipstream_shared::error::SlipstreamResult;

pub mod info;
pub mod intro;
pub mod panic;
pub mod settings;
pub mod splash;

/// A page that can be displayed by the editor and which can be routed to.
///
/// A new routable page can be sent to the app via the [`Route`] command.
///
/// [`Route`]: crate::app::AppCommand::Route
pub trait RoutablePage: Send {
    fn name(&self) -> &str;
    fn draw(&mut self, ui: &mut egui::Ui) -> SlipstreamResult<()>;

    fn update(&mut self) -> SlipstreamResult<()> {
        Ok(())
    }
}
