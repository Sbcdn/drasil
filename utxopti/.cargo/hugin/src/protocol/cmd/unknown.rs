use crate::{Connection, Frame};
use tracing::{debug, instrument};

#[derive(Debug, Clone)]
pub struct Unknown {
    command_name: String,
}

impl Unknown {
    pub(crate) fn new(key: impl ToString) -> Unknown {
        Unknown {
            command_name: key.to_string(),
        }
    }

    pub(crate) fn get_name(&self) -> &str {
        &self.command_name
    }

    #[instrument(skip(self, dst))]
    pub(crate) async fn apply(self, dst: &mut Connection) -> crate::Result<()> {
        let response = Frame::Error(format!("ERROR: unknown command '{}'", self.command_name));
        debug!(?response);
        dst.write_frame(&response).await?;
        Ok(())
    }
}
