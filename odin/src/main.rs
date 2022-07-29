extern crate pretty_env_logger;
extern crate diesel;
use hugin::Command;
use hugin::protocol::{connection::Connection, Shutdown};

use std::future::Future;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc, Semaphore};
use tokio::time::{self, Duration};

use std::str;
use std::env;


struct Listener {
    listener : TcpListener,
    limit_connections: Arc<Semaphore>,

    notify_shutdown: broadcast::Sender<()>,
    shutdown_complete_rx: mpsc::Receiver<()>,
    shutdown_complete_tx: mpsc::Sender<()>,
}

struct Handler {
    connection : Connection,
    limit_connections : Arc<Semaphore>,

    shutdown: Shutdown,
    _shutdown_complete: mpsc::Sender<()>,
}

const DEFAULT_HOST : &str = "127.0.0.1";
const DEFAULT_PORT : &str = "6142";
const MAX_CONNECTIONS: usize = 1000;

pub async fn run(listener: TcpListener, shutdown: impl Future) -> crate::Result<()> {
    let (notify_shutdown, _) = broadcast::channel(1);
    let (shutdown_complete_tx,shutdown_complete_rx) = mpsc::channel(1);
    let mut server = Listener {
        listener,
        limit_connections: Arc::new(Semaphore::new(MAX_CONNECTIONS)),
        notify_shutdown,
        shutdown_complete_tx,
        shutdown_complete_rx
    };

    tokio::select! {
        res = server.run() => {
            if let Err(err) = res {
                log::error!("failed to accept: {:?}",err);
            }   
        }
        _ = shutdown => {
            log::info!("shutting down")
        }
    }

    let Listener {
        mut shutdown_complete_rx,
        shutdown_complete_tx,
        notify_shutdown,
        ..
    } = server;

    drop(notify_shutdown);
    drop(shutdown_complete_tx);
    let _ = shutdown_complete_rx.recv().await;

    Ok(())
}


impl Listener {
    async fn run(&mut self) -> crate::Result<()> {
        log::info!("accepting inbound connections at {:?}",self.listener.local_addr()?);

        loop {
            self.limit_connections.acquire().await?.forget();
            let socket = self.accept().await?;
            let mut handler = Handler {
                connection: Connection::new(socket),
                limit_connections: self.limit_connections.clone(),
                shutdown: Shutdown::new(self.notify_shutdown.subscribe()),
                _shutdown_complete: self.shutdown_complete_tx.clone(),
            };
            tokio::spawn(async move {
                if let Err(err) = handler.run().await {
                    log::error!("connection error: {:?}",err);
                }
            });

        }
    } 
    
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
            log::debug!("CMD: {:?}",cmd);
            cmd?.apply(&mut self.connection, &mut self.shutdown)
                .await?;
        }

        Ok(())

    }
}

impl Drop for Handler {
    fn drop(&mut self) {
        self.limit_connections.add_permits(1);   
    }
}

use tokio::signal;
#[tokio::main]
pub async fn main() -> crate::Result<()> {
    pretty_env_logger::init();
    let host : String =  env::var("POD_HOST").unwrap_or(DEFAULT_HOST.to_string());
    let port = env::var("POD_PORT").unwrap_or(DEFAULT_PORT.to_string());
    let listener = TcpListener::bind(&format!("{}:{}",host,port)).await?;
    
    run(listener, signal::ctrl_c()).await
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;