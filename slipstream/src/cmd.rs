use futures::{SinkExt, channel::mpsc};

use crate::{error::SlipstreamResult, pages::RoutablePage};

/// A command that can be executed by the root app.
pub enum AppCommand {
    /// Changes the currently displayed page to the given one.
    Route(Box<dyn RoutablePage>),
    CenterWindow,
}

/// A message channel used to make structural changes to the app, that cannot be performed
/// while drawing UI.
#[derive(Clone)]
pub struct AppCommandChannel {
    sender: mpsc::Sender<AppCommand>,
}

impl AppCommandChannel {
    pub fn new(sender: mpsc::Sender<AppCommand>) -> Self {
        Self { sender }
    }

    /// Attempts to change the current page to the given one,
    /// returning an error if the channel was full or closed.
    pub fn try_route(&mut self, page_data: Box<dyn RoutablePage>) -> SlipstreamResult<()> {
        self.sender.try_send(AppCommand::Route(page_data))?;
        Ok(())
    }

    /// Sends the command to change the current page to the given one,
    /// blocking until capacity is available to send the message.
    pub async fn route(&mut self, page_data: Box<dyn RoutablePage>) -> SlipstreamResult<()> {
        // Use send to force the channel to be flushed.
        self.sender.send(AppCommand::Route(page_data)).await?;
        Ok(())
    }

    /// Puts the window in the center of the screen.
    ///
    /// This function returns an error if the channel has been closed or is full.
    pub fn try_center_window(&mut self) -> SlipstreamResult<()> {
        self.sender.try_send(AppCommand::CenterWindow)?;
        Ok(())
    }

    /// Puts the window in the center of the screen.
    ///
    /// This function blocks until capacity is available in the channel.
    pub async fn center_window(&mut self) -> SlipstreamResult<()> {
        self.sender.send(AppCommand::CenterWindow).await?;
        Ok(())
    }
}
