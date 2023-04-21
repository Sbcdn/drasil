/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/

use crate::models::{Client, Clients, WSCom};
use deadpool_lapin::Pool;
use futures::{FutureExt, StreamExt};
use ratelimit_meter::{DirectRateLimiter, LeakyBucket};
use std::convert::Infallible;
use tokio::sync::mpsc;
use tokio::time::Duration;
use tokio_stream::wrappers::UnboundedReceiverStream;
use uuid::Uuid;
use warp::ws::{Message, WebSocket};

pub async fn main_worker(clients: Clients) {
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
        let connected_client_count = clients.lock().await.len();
        if connected_client_count == 0 {
            println!("No clients connected, skip sending data");
            continue;
        }
        println!("{connected_client_count} connected client(s)");

        clients.lock().await.iter().for_each(|(_, client)| {
            if let Some(sender) = &client.sender {
                let _ = sender.send(Ok(Message::binary(
                    serde_json::to_string(&format!("Hello user {:?}", client.client_id)).unwrap(),
                )));
            }
        });
    }
}

pub(crate) async fn handle_ws_client(
    user_id: u64,
    ws: warp::ws::Ws,
    clients: Clients,
    pool: Pool,
    rate_limiter: DirectRateLimiter<LeakyBucket>,
) -> Result<impl warp::Reply, Infallible> {
    println!("ws_handler");
    Ok(
        ws.on_upgrade(move |socket| {
            client_connection(user_id, socket, clients, pool, rate_limiter)
        }),
    )
}

async fn client_connection(
    user_id: u64,
    ws: WebSocket,
    clients: Clients,
    pool: Pool,
    rate_limiter: DirectRateLimiter<LeakyBucket>,
) {
    println!("establishing client connection... {ws:?}");

    let (client_ws_sender, mut client_ws_rcv) = ws.split();
    let (client_sender, client_rcv) = mpsc::unbounded_channel();
    let client_rcv = UnboundedReceiverStream::new(client_rcv);
    tokio::task::spawn(client_rcv.forward(client_ws_sender).map(|result| {
        if let Err(e) = result {
            println!("error sending websocket msg: {e}");
        }
    }));

    let uuid = Uuid::new_v4().as_simple().to_string();

    let new_client = Client {
        client_id: uuid.clone(),
        sender: Some(client_sender),
        user_id,
    };
    clients.lock().await.insert(uuid.clone(), new_client);

    while let Some(result) = client_ws_rcv.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                println!("error receiving message for id {}): {}", uuid.clone(), e);
                break;
            }
        };
        client_msg(
            uuid.clone(),
            user_id as i64,
            msg,
            &clients,
            pool.clone(),
            &mut rate_limiter.clone(),
        )
        .await;
    }
    clients.lock().await.remove(&uuid);
    println!("{uuid} disconnected");
}

async fn client_msg(
    client_id: String,
    user_id: i64,
    msg: Message,
    clients: &Clients,
    pool: Pool,
    rate_limiter: &mut DirectRateLimiter<LeakyBucket>,
) {
    println!("received message from {client_id}: {msg:?}");
    let message = match msg.to_str() {
        Ok(v) => v,
        Err(_) => return,
    };

    match serde_json::from_str(message).unwrap() {
        //"alive" | "alive\n"
        WSCom::Alive => {
            let locked = clients.lock().await;
            if let Some(v) = locked.get(&client_id) {
                if let Some(sender) = &v.sender {
                    log::info!("sending alive");
                    let _ = sender.send(Ok(Message::text("OK")));
                }
            }
        }
        //"new_token"
        WSCom::ClaimMintRewards(mut cmr) => {
            let locked = clients.lock().await;
            if let Some(v) = locked.get(&client_id) {
                if let Some(sender) = &v.sender {
                    log::info!("Try to claim mint reward...");
                    // Send Requst into Queue and respond with waiting time
                    cmr.user_id = Some(user_id);
                    match super::add_msg_handler(pool, &cmr, rate_limiter).await {
                        Ok(o) => {
                            let _ = sender.send(Ok(Message::binary(o)));
                        }
                        Err(e) => {
                            log::error!("Error adding message handler: {:?}", e);
                            let _ = sender.send(Ok(Message::binary(
                                serde_json::to_string("too many requests").unwrap(),
                            )));
                        }
                    }
                }
            }
        } //   _ => (),
    }
}
