/// Odin is a server that receives transaction requests from Heimdallr clients. Odin can establish a TCP
/// connection with a given number of Heimdallr clients. Each Heimdallr client can send a transaction
/// request through the TCP connection to Odin server, which will cause the Odin server to 
/// parse the transaction request from compressed form into human-readable form (also the form expected by 
/// other parts of Drasil). The parsed transaction request is then passed to Hugin for further processing.  

extern crate pretty_env_logger;
use drasil_hugin::protocol::{connection::Connection, Shutdown};
use drasil_hugin::Command;

use std::future::Future;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc, Semaphore};
use tokio::time::{self, Duration};

use std::env;
use std::str;

/// Wrapper that extends the server (`TcpListener`) with additional network capabilities.  
struct Listener {
    /// basic server implementation
    listener: TcpListener,
    /// maximal number of clients that can be simultaneously connected to the server
    limit_connections: Arc<Semaphore>,

    notify_shutdown: broadcast::Sender<()>,
    shutdown_complete_rx: mpsc::Receiver<()>,
    shutdown_complete_tx: mpsc::Sender<()>,
}

/// Wrapper around `Connection` connection that prevents the number of connections from exceeding
/// a maximal number, and enables the checking of whether the connection is alive. 
struct Handler {
    connection: Connection,
    limit_connections: Arc<Semaphore>,

    shutdown: Shutdown,
    _shutdown_complete: mpsc::Sender<()>,
}

/// Default address exposed by Odin server if another address isn't specified. 
const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: &str = "6142";
const MAX_CONNECTIONS: usize = 1000;

/// Run the Odin server until Odin receives ctrl_c shutdown command.
pub async fn run(listener: TcpListener, shutdown: impl Future) -> crate::Result<()> {
    let (notify_shutdown, _) = broadcast::channel(1);
    let (shutdown_complete_tx, shutdown_complete_rx) = mpsc::channel(1);
    let mut server = Listener {
        listener,
        limit_connections: Arc::new(Semaphore::new(MAX_CONNECTIONS)),
        notify_shutdown,
        shutdown_complete_tx,
        shutdown_complete_rx,
    };

    // Run Odin server, until Odin receives ctrl+C which causes the server to turn off
    tokio::select! {
        res = server.run() => {
            if let Err(err) = res {
                log::error!("failed to accept: {:?}", err);
            }
        }
        _ = shutdown => {
            log::info!("shutting down")
        }
    }

    // Separate out Odin's channels in preparation for shutdown
    let Listener {
        mut shutdown_complete_rx,
        shutdown_complete_tx,
        notify_shutdown,
        ..
    } = server;

    // Shut down the broadcast & mpsc channels
    drop(notify_shutdown);
    drop(shutdown_complete_tx);
    let _ = shutdown_complete_rx.recv().await;

    Ok(())
}

impl Listener {
    /// Run Odin server by establishing connections between Odin server 
    /// and (one or) many remote clients. 
    async fn run(&mut self) -> crate::Result<()> {
        log::info!(
            "accepting inbound connections at {:?}",
            self.listener.local_addr()?
        );

        // each iteration represents a single connection
        loop {
            self.limit_connections.acquire().await?.forget();
            let socket = self.accept().await?;
            log::debug!("From peer address: {:?}", socket.peer_addr().unwrap());
            let mut handler = Handler {
                connection: Connection::new(socket),
                limit_connections: self.limit_connections.clone(),
                shutdown: Shutdown::new(self.notify_shutdown.subscribe()),
                _shutdown_complete: self.shutdown_complete_tx.clone(),
            };

            tokio::spawn(async move {
                if let Err(err) = handler.run().await {
                    log::error!("connection error: {:?}", err);
                }
            });
        }
    }

    /// Accept incoming connection from remote client. This function checks for
    /// connection requests over and over. If remote client hasn't made any connection 
    /// requests, this function will wait twice as long as last time before checking again. 
    async fn accept(&mut self) -> crate::Result<TcpStream> {
        log::info!("accepted connection");
        let mut backoff = 1;
        loop {
            match self.listener.accept().await {
                Ok((socket, _)) => return Ok(socket),
                Err(err) => {
                    if backoff > 64 {
                        return Err(err.into());
                    }
                }
            }
            time::sleep(Duration::from_secs(backoff)).await;
            backoff *= 2;
        }
    }
}

impl Handler {
    /// Continuously listen for incoming instructions from the given remote client
    /// to Odin server until the connection is shut down. 
    async fn run(&mut self) -> crate::Result<()> {
        log::debug!("started new handler");
        while !self.shutdown.is_shutdown() {
            let maybe_frame = tokio::select! {
                res = self.connection.read_frame() => res?,
                _ = self.shutdown.recv() => {
                    return Ok(());
                }
            };
            let frame = match maybe_frame {
                Some(frame) => frame,
                None => return Ok(()),
            };
            let cmd = Command::from_frame(frame);
            log::debug!("CMD: {:?}", cmd);
            cmd?.apply(&mut self.connection, &mut self.shutdown).await?;
        }

        Ok(())
    }
}

/// Increase the number of allowed connections to Odin server by 1 when 
/// current connection is terminated
impl Drop for Handler {
    fn drop(&mut self) {
        self.limit_connections.add_permits(1);
    }
}

/// Specify the address that Odin server will expose, and then run the Odin server. 
use tokio::signal;
#[tokio::main]
pub async fn main() -> crate::Result<()> {
    pretty_env_logger::init();
    let host: String = env::var("POD_HOST").unwrap_or_else(|_| DEFAULT_HOST.to_string());
    let port = env::var("POD_PORT").unwrap_or_else(|_| DEFAULT_PORT.to_string());
    let listener = TcpListener::bind(&format!("{host}:{port}")).await?;

    run(listener, signal::ctrl_c()).await
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;
