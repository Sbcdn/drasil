use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, Mutex};
use warp::{ws::Message, Filter};

#[derive(Debug, Clone)]
pub struct Client {
    pub client_id: String,
    pub user_id: u64,
    pub sender: Option<mpsc::UnboundedSender<std::result::Result<Message, warp::Error>>>,
}

type Clients = Arc<Mutex<HashMap<String, Client>>>;
mod error;

///Filters
mod filters {
    use super::handlers;
    use super::Clients;
    use serde::Serialize;
    use std::convert::Infallible;
    use warp::{hyper::StatusCode, Filter};
    #[derive(Serialize, Debug)]
    struct ErrorResult {
        detail: String,
    }

    pub fn endpoints(
        clients: Clients,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        websocket(clients).or(alive()).or(resp_option())
    }

    pub fn resp_option() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        warp::options()
            .and(warp::header("origin"))
            .map(|origin: String| {
                Ok(warp::http::Response::builder()
                    .status(warp::http::StatusCode::OK)
                    .header("access-control-allow-methods", "HEAD, GET, POST, OPTION")
                    .header("access-control-allow-headers", "authorization")
                    .header("access-control-allow-credentials", "true")
                    .header("access-control-max-age", "300")
                    .header("access-control-allow-origin", origin)
                    .header("vary", "origin")
                    .body(""))
            })
    }

    /// GET contracts
    pub fn alive() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("alive")
            .and(warp::get())
            .and(warp::any().map(warp::reply))
    }

    pub fn websocket(
        clients: Clients,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path("auth")
            .and(auth())
            .and(warp::ws())
            .and(with_clients(clients))
            .and_then(handlers::handle_ws_client)
        //.and_then(ws.on_upgrade(handlers::handle_ws_client))
    }

    fn with_clients(
        clients: Clients,
    ) -> impl Filter<Extract = (Clients,), Error = Infallible> + Clone {
        warp::any().map(move || clients.clone())
    }

    pub(crate) fn auth() -> impl Filter<Extract = (u64,), Error = warp::Rejection> + Clone {
        use super::auth::authorize;
        use warp::{
            filters::header::headers_cloned,
            http::header::{HeaderMap, HeaderValue},
        };
        headers_cloned()
            .map(move |headers: HeaderMap<HeaderValue>| (headers))
            //.and(bytes().map(move |body: bytes::Bytes| (body)))
            .and_then(authorize)
    }

    pub(crate) async fn handle_rejection(
        err: warp::reject::Rejection,
    ) -> std::result::Result<impl warp::reply::Reply, Infallible> {
        let code;
        let message;

        if err.is_not_found() {
            code = StatusCode::NOT_FOUND;
            message = "Not found";
        } else if err
            .find::<warp::filters::body::BodyDeserializeError>()
            .is_some()
        {
            code = StatusCode::BAD_REQUEST;
            message = "Invalid Body";
        } else if let Some(e) = err.find::<crate::error::Error>() {
            match e {
                crate::error::Error::NotAuthorized(_error_message) => {
                    code = StatusCode::UNAUTHORIZED;
                    message = "Action not authorized";
                }
                crate::error::Error::JWTTokenError => {
                    code = StatusCode::BAD_GATEWAY;
                    message = "Action not authorized";
                }
                crate::error::Error::NoAuthHeaderError => {
                    code = StatusCode::BAD_REQUEST;
                    message = "No authentication";
                }
                crate::error::Error::InvalidAuthHeaderError => {
                    code = StatusCode::BAD_REQUEST;
                    message = "Invalid authentication";
                }
                crate::error::Error::Custom(_) => {
                    code = StatusCode::INTERNAL_SERVER_ERROR;
                    message = "Internal Error";
                }
                crate::error::Error::JWTTokenCreationError => {
                    code = StatusCode::INTERNAL_SERVER_ERROR;
                    message = "Token Creation Error";
                }
            }
        } else if err.find::<warp::reject::MethodNotAllowed>().is_some() {
            code = StatusCode::METHOD_NOT_ALLOWED;
            message = "Method not allowed";
        } else {
            // We should have expected this... Just log and say its a 500
            log::error!("unhandled rejection: {:?}", err);
            code = StatusCode::INTERNAL_SERVER_ERROR;
            message = "Internal server error";
        }

        let json = warp::reply::json(&ErrorResult {
            detail: message.into(),
        });

        Ok(warp::reply::with_status(json, code))
    }
}

mod auth {
    use crate::error::{self, Error};
    use chrono::prelude::*;
    use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
    use serde::{Deserialize, Serialize};

    use warp::{
        http::header::{HeaderMap, HeaderValue, AUTHORIZATION},
        reject, Rejection,
    };

    const BEARER: &str = "Bearer ";

    #[derive(Debug, Deserialize, Serialize)]
    struct ApiClaims {
        sub: String,
        exp: usize,
    }

    pub(crate) async fn authorize(
        headers: HeaderMap<HeaderValue>,
        //body: bytes::Bytes,
    ) -> Result<u64, Rejection> {
        let publ = std::env::var("JWT_PUB_KEY")
            .map_err(|_| Error::Custom("env jwt pub not existing".to_string()))?;
        let publ = publ.into_bytes();
        log::info!("checking login data ...");

        match jwt_from_header(&headers) {
            Ok(jwt) => {
                let decoded = decode::<ApiClaims>(
                    &jwt,
                    &DecodingKey::from_ec_pem(&publ).unwrap(),
                    &Validation::new(Algorithm::ES256),
                )
                .map_err(|_| reject::custom(Error::JWTTokenError))?;
                log::info!("lookup user data ...");
                let user_id = decoded.claims.sub.parse::<u64>().map_err(|_| {
                    reject::custom(Error::Custom("Could not parse customer id".to_string()))
                })?;
                //let mut client = connect(std::env::var("ODIN_URL").unwrap()).await.unwrap();
                //let cmd = VerifyUser::new(user_id, jwt);
                //log::info!("try to verify user ...");
                //match client.build_cmd::<VerifyUser>(cmd).await {
                //    Ok(_) => {}
                //    Err(_) => {
                //       return Err(reject::custom(Error::JWTTokenError));
                //   }
                // };
                log::info!("Authentication successful: User_id: {:?}", user_id);
                Ok(user_id)
            }

            Err(e) => {
                log::info!("Authentication not successful");
                Err(reject::custom(e))
            }
        }
    }

    fn jwt_from_header(headers: &HeaderMap<HeaderValue>) -> Result<String, error::Error> {
        let header = match headers.get(AUTHORIZATION) {
            Some(v) => v,
            None => return Err(Error::NoAuthHeaderError),
        };
        let auth_header = match std::str::from_utf8(header.as_bytes()) {
            Ok(v) => v,
            Err(_) => return Err(Error::NoAuthHeaderError),
        };
        if !auth_header.starts_with(BEARER) {
            return Err(Error::InvalidAuthHeaderError);
        }
        Ok(auth_header.trim_start_matches(BEARER).to_owned())
    }

    pub fn create_jwt(uid: &str) -> Result<String, error::Error> {
        let expiration = Utc::now()
            .checked_add_signed(chrono::Duration::seconds(1800))
            .expect("valid timestamp")
            .timestamp();

        let claims = ApiClaims {
            sub: uid.to_owned(),
            exp: expiration as usize,
        };
        let header = Header::new(Algorithm::ES256);
        let key = std::env::var("JWT_KEY")
            .map_err(|_| Error::Custom("env jwt key path not existing".to_string()))?;
        let key = key.into_bytes(); //std::fs::read(key).expect("Could not read jwt key file");
        encode(&header, &claims, &EncodingKey::from_ec_pem(&key).unwrap())
            .map_err(|_| Error::JWTTokenCreationError)
    }
}

mod handlers {
    use crate::{auth, error};

    use super::{Client, Clients};
    use chrono::{DateTime, Utc};
    use futures::{FutureExt, StreamExt};
    use serde::Serialize;
    use std::convert::Infallible;
    use tokio::sync::mpsc;
    use tokio::time::Duration;
    use tokio_stream::wrappers::UnboundedReceiverStream;
    use uuid::Uuid;
    use warp::ws::{Message, WebSocket};

    #[derive(Serialize)]
    struct TestData {
        widget_type: String,
        widget_count: u32,
        creation_date: DateTime<Utc>,
    }

    #[derive(Serialize)]
    struct JWTToken {
        token: String,
        creation_date: DateTime<Utc>,
    }

    fn generate_new_token(client: &Client) -> Result<JWTToken, error::Error> {
        let token = auth::create_jwt(&client.user_id.to_string())?;
        Ok(JWTToken {
            token,
            creation_date: Utc::now(),
        })
    }

    pub async fn main_worker(clients: Clients) {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            let connected_client_count = clients.lock().await.len();
            if connected_client_count == 0 {
                println!("No clients connected, skip sending data");
                continue;
            }
            log::debug!("{} connected client(s)", connected_client_count);

            clients.lock().await.iter().for_each(|(_, client)| {
                if let Some(sender) = &client.sender {
                    let _ = sender.send(Ok(Message::binary(
                        serde_json::to_string(&generate_new_token(client).unwrap()).unwrap(),
                    )));
                }
            });
        }
    }

    pub(crate) async fn handle_ws_client(
        user_id: u64,
        ws: warp::ws::Ws,
        clients: Clients,
    ) -> Result<impl warp::Reply, Infallible> {
        log::debug!("ws_handler");
        Ok(ws.on_upgrade(move |socket| client_connection(user_id, socket, clients)))
        //
    }

    async fn client_connection(user_id: u64, ws: WebSocket, clients: Clients) {
        log::debug!("establishing client connection... {:?}", ws);

        let (client_ws_sender, mut client_ws_rcv) = ws.split();
        let (client_sender, client_rcv) = mpsc::unbounded_channel();
        let client_rcv = UnboundedReceiverStream::new(client_rcv);
        tokio::task::spawn(client_rcv.forward(client_ws_sender).map(|result| {
            if let Err(e) = result {
                log::debug!("error sending websocket msg: {}", e);
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
            client_msg(uuid.clone(), msg, &clients).await;
        }
        clients.lock().await.remove(&uuid);
        log::debug!("{} disconnected", uuid);
    }

    async fn client_msg(client_id: String, msg: Message, clients: &Clients) {
        log::debug!("received message from {}: {:?}", client_id, msg);
        let message = match msg.to_str() {
            Ok(v) => v,
            Err(_) => return,
        };
        if message == "alive" || message == "alive\n" {
            let locked = clients.lock().await;
            if let Some(v) = locked.get(&client_id) {
                if let Some(sender) = &v.sender {
                    log::info!("sending alive");
                    let _ = sender.send(Ok(Message::text("yes")));
                }
            }
        };
        if message == "new_token" {
            let locked = clients.lock().await;
            if let Some(v) = locked.get(&client_id) {
                if let Some(sender) = &v.sender {
                    log::info!("sending alive");
                    let _ = sender.send(Ok(Message::binary(
                        serde_json::to_string(&generate_new_token(v).unwrap()).unwrap(),
                    )));
                }
            }
        };
    }
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    let clients: Clients = Arc::new(Mutex::new(HashMap::new()));
    log::info!("Configuring websocket route");

    let api = filters::endpoints(clients.clone());

    let routes = api
        .with(warp::cors().allow_any_origin())
        .recover(filters::handle_rejection)
        .with(warp::log("pauther"));
    log::info!("Starting update loop");
    tokio::task::spawn(async move {
        handlers::main_worker(clients.clone()).await;
    });
    log::info!("Starting server");

    warp::serve(routes).run(([127, 0, 0, 1], 8000)).await;
}
