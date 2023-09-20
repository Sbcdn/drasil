use lazy_static::lazy_static;
use std::env::{set_var, var};
use std::io::{Read, Write};
use vaultrs::api::auth::approle::responses::GenerateNewSecretIDResponse;
use vaultrs::api::ResponseWrapper;
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};
use vaultrs::*;
use vaultrs::{api::AuthInfo, client::Client};

use hyper::Client as HClient;
use hyperlocal::{UnixClientExt, Uri};

lazy_static! {
    static ref VROLE_ID: String = var("VROLE_ID").unwrap_or_else(|_| "dummy_role".to_string());
    static ref VROLE_NAME: String = var("VROLE_NAME").unwrap_or_else(|_| "dummy_role".to_string());
    static ref VSECRET_ID: String =
        var("VSECRET_ID").unwrap_or_else(|_| "dummy_secret".to_string());
    static ref VAULT_ADDR: String =
        var("VAULT_ADDR").unwrap_or_else(|_| "dummy_address".to_string());
    static ref VAULT_NAMESPACE: String =
        var("VAULT_NAMESPACE").unwrap_or_else(|_| "dummy_ns".to_string());
    static ref MOUNT: String = var("MOUNT").unwrap_or_else(|_| "dummy_mount".to_string());
    static ref SPATH: String = var("SPATH").unwrap_or_else(|_| "dummy_path".to_string());
    static ref VSOCKET_PATH: String =
        var("VSOCKET_PATH").unwrap_or_else(|_| "dummy_path".to_string());
    static ref VPATH: String = var("VPATH").unwrap_or_else(|_| "dummy_path".to_string());
}

fn get_role_id() -> String {
    VROLE_ID.to_string()
}

fn get_role_name() -> String {
    VROLE_NAME.to_string()
}

fn get_secret_id() -> String {
    VSECRET_ID.to_string()
}

fn get_vault_address() -> String {
    VAULT_ADDR.to_string()
}

fn get_namespace() -> String {
    VAULT_NAMESPACE.to_string()
}

pub(crate) fn get_gl_mount() -> String {
    MOUNT.to_string()
}

pub(crate) fn get_v_path() -> String {
    VPATH.to_string()
}

fn get_secret_path() -> String {
    SPATH.to_string()
}

fn get_vtoken() -> String {
    match std::env::var("VAULT_TOKEN") {
        Ok(o) => o,
        Err(_) => "".to_string(),
    }
}

async fn get_own_vault_token(client: &mut VaultClient) -> String {
    match std::env::var("VAULT_TOKEN") {
        Ok(o) => {
            client.set_token(&o);
            match vaultrs::token::lookup_self(client).await {
                Ok(lr) => {
                    // maximum time to life allowed for the token
                    if lr.renewable && lr.explicit_max_ttl > (lr.ttl + 10) && lr.ttl < 10 {
                        let auth = vaultrs::token::renew_self(client, None).await.unwrap();
                        auth.client_token
                    } else if lr.ttl > 10 {
                        o
                    } else {
                        vault_auth(client).await.client_token
                    }
                }
                Err(_) => vault_auth(client).await.client_token,
            }
        }
        Err(_) => vault_auth(client).await.client_token,
    }
}

async fn login(client: &mut VaultClient) {
    match std::env::var("VAULT_TOKEN") {
        Ok(o) => {
            client.set_token(&o);
            match vaultrs::token::lookup_self(client).await {
                Ok(lr) => {
                    // maximum time to life allowed for the token
                    if lr.renewable && lr.explicit_max_ttl > (lr.ttl + 10) && lr.ttl < 10 {
                        let auth = vaultrs::token::renew_self(client, None).await.unwrap();
                        client.set_token(&auth.client_token);
                    } else if lr.ttl > 10 {
                    } else {
                        renew_token(client, &get_role_id()).await;
                    }
                }
                Err(_) => {
                    renew_token(client, &get_role_id()).await;
                }
            }
        }
        Err(_) => {
            renew_token(client, &get_role_id()).await;
        }
    }
}

async fn request_secret(
    vclient: &mut VaultClient,
    path: &String,
) -> Result<GenerateNewSecretIDResponse, crate::error::Error> {
    let url = Uri::new(
        VSOCKET_PATH.to_string(),
        &("/auth/".to_string() + &get_role_name()),
    )
    .into();

    let client = HClient::unix();

    let mut response = client.get(url);

    match tokio::time::timeout(std::time::Duration::from_secs(1), &mut response).await {
        Err(_) => {
            log::error!("secret request timeout");
        }
        Ok(no_timeout) => match no_timeout {
            Ok(resp) => {
                log::debug!("Response: {:?}", resp);
                let r_status = resp.status();
                //let resp_text = resp.text().await.unwrap();
                if r_status != http::StatusCode::ACCEPTED {
                    log::error!("secret request not accepted");
                } else {
                    return Ok(read_wrapped_secret(vclient, path).await);
                }
            }
            Err(_) => {
                log::error!("error on secret request");
            }
        },
    }
    Err(crate::error::Error::StdError)
}

pub async fn renew_token(client: &mut VaultClient, role_id: &str) -> String {
    let secret = &request_secret(client, &get_secret_path()).await.unwrap();
    let wtoken = obtain_token(client, role_id, &secret.secret_id).await;
    set_vault_token(client, &wtoken).await
}

async fn obtain_token(client: &mut VaultClient, role_id: &str, secret_id: &str) -> AuthInfo {
    vaultrs::auth::approle::login(client, "approle", role_id, secret_id)
        .await
        .unwrap()
}

async fn set_vault_token(client: &mut VaultClient, auth: &AuthInfo) -> String {
    set_var("VAULT_TOKEN", auth.client_token.clone());
    client.set_token(&auth.client_token);
    auth.client_token.clone()
}

async fn vault_auth(client: &VaultClient) -> AuthInfo {
    let secret_id = get_secret_id();
    let role_id = get_role_id();
    vaultrs::auth::approle::login(client, "approle", &role_id, &secret_id)
        .await
        .unwrap()
}

pub async fn vault_connect_sdc() -> VaultClient {
    let address = get_vault_address();
    let namespace = get_namespace();
    let mut client = VaultClient::new(
        VaultClientSettingsBuilder::default()
            .address(address)
            .set_namespace(namespace)
            .token("")
            .timeout(Some(std::time::Duration::from_secs(30)))
            .build()
            .unwrap(),
    )
    .unwrap();

    let token = get_own_vault_token(&mut client).await;
    client.set_token(&token);
    std::env::set_var("VAULT_TOKEN", &token);
    client
}

pub async fn vault_connect() -> VaultClient {
    let address = get_vault_address();
    let namespace = get_namespace();
    let mut client = VaultClient::new(
        VaultClientSettingsBuilder::default()
            .address(address)
            .set_namespace(namespace)
            .token(get_vtoken())
            .timeout(Some(std::time::Duration::from_secs(30)))
            .build()
            .unwrap(),
    )
    .unwrap();

    login(&mut client).await;
    client
}

async fn get_wrapped_secret_id(client: &VaultClient, role_id: &str) -> String {
    let mut t = api::auth::approle::requests::GenerateNewSecretIDRequest::builder();
    let endpoint = t.mount("approle").role_name(role_id).build().unwrap(); //mount(&get_gl_mount())
    let result = endpoint.wrap(client).await.unwrap(); //api::wrap(client, endpoint).await.unwrap();
    log::info!("Got wrapped token: {:?}", result.info);
    result.info.token
}

async fn lstore_wrapper_token(token: &String, file: &String) {
    let mut file =
        std::fs::File::create(file).unwrap_or_else(|_| panic!("Could not create file: {file}"));
    file.write_all(token.as_bytes())
        .expect("Could not write to file");
}

pub async fn store_wrapped_secret(role_id: &str) {
    let client = vault_connect_sdc().await;
    let ws = get_wrapped_secret_id(&client, role_id).await;
    lstore_wrapper_token(&ws, &get_secret_path()).await
}

async fn read_wrapped_secret(
    client: &mut VaultClient,
    path: &String,
) -> GenerateNewSecretIDResponse {
    let mut file = std::fs::File::open(path).unwrap();
    let mut token = String::new();
    file.read_to_string(&mut token).unwrap();
    std::fs::remove_file(path).expect("could not remove file");
    client.set_token(&token);
    log::info!("Secret delivered");
    vaultrs::sys::wrapping::unwrap(client, None).await.unwrap()
}
