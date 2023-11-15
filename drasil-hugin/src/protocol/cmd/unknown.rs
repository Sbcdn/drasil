/// This is a placeholder for future implementation.

use crate::{Connection, Frame};

/// The parsed data attached to the incoming command that requests an action that hasn't been implemented yet. 
#[derive(Debug, Clone)]
pub struct Unknown {
    command_name: String,
}

/// Placeholder for future implementation
impl Unknown {
    pub(crate) fn new(key: impl ToString) -> Unknown {
        unimplemented!();
        Unknown {
            command_name: key.to_string(),
        }
    }

    pub(crate) fn _get_name(&self) -> &str {
        unimplemented!();
        &self.command_name
    }

    /// Perform the unimplemented action requested by the user. `Unknown` (`self`) contains the building blocks used in this method.
    /// `dst` is the connection to the Heimdallr client (and thus indirectly to the user) who requested the unimplemented action. 
    /// This method sends a response back to this Heimdallr client (and thus back to the user who requested 
    /// the unimplemented action). 
    pub(crate) async fn apply(self, dst: &mut Connection) -> crate::Result<()> {
        unimplemented!();
        let response = Frame::Error(format!("ERROR: unknown command '{}'", self.command_name));
        log::debug!("{:?}", response);
        dst.write_frame(&response).await?;
        Ok(())
    }
}
