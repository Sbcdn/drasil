use crate::{Connection, Frame};

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

    pub(crate) fn _get_name(&self) -> &str {
        &self.command_name
    }

    pub(crate) async fn apply(self, dst: &mut Connection) -> crate::Result<()> {
        let response = Frame::Error(format!("ERROR: unknown command '{}'", self.command_name));
        log::debug!("{:?}", response);
        dst.write_frame(&response).await?;
        Ok(())
    }
}
