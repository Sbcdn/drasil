use tokio::sync::broadcast;

/// Specifies the state of a given TCP connection. Can be used as 
/// a trigger mechanism for whether the server should continue 
/// listening to a TCP connection or drop it.
#[derive(Debug)]
pub struct Shutdown {
    shutdown: bool,
    notify: broadcast::Receiver<()>,
}

impl Shutdown {
    pub fn new(notify: broadcast::Receiver<()>) -> Shutdown {
        Shutdown {
            shutdown: false,
            notify,
        }
    }

    pub fn is_shutdown(&self) -> bool {
        self.shutdown
    }

    pub async fn recv(&mut self) {
        if self.shutdown {
            return;
        }
        let _ = self.notify.recv().await;
        self.shutdown = true;
    }
}
