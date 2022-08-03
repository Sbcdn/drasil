/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use chacha20poly1305::{
    aead::{stream, NewAead},
    XChaCha20Poly1305,
};
use mimir::MurinError;
use rand::{rngs::OsRng, RngCore};

use serde_json::json;
use sha2::Digest;
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};
use vaultrs::{api::AuthInfo, client::Client};

use std::{
    collections::HashMap,
    io::{BufWriter, Read, Write},
};
use zeroize::Zeroize;

fn argon2_config<'a>() -> rargon2::Config<'a> {
    rargon2::Config {
        variant: rargon2::Variant::Argon2id,
        hash_length: 32,
        lanes: 8,
        mem_cost: 16 * 1024,
        time_cost: 8,
        ..Default::default()
    }
}

fn get_secure_key_from_pwd(pwd: &String) -> (Vec<u8>, [u8; 32]) {
    let mut salt = [0u8; 32];
    OsRng.fill_bytes(&mut salt);
    let argon2_config = argon2_config();
    let key = rargon2::hash_raw(pwd.as_bytes(), &salt, &argon2_config)
        .expect("Could not create encryption key");

    (key, salt)
}

async fn vault_auth(client: &VaultClient) -> AuthInfo {
    let secret_id = std::env::var("VSECRET_ID").unwrap();
    let role_id = std::env::var("V_ROLE_ID").unwrap();
    vaultrs::auth::approle::login(client, "approle", &role_id, &secret_id)
        .await
        .unwrap()
}

pub async fn vault_connect() -> VaultClient {
    let address = std::env::var("VAULT_ADDR").unwrap();
    let namespace = std::env::var("VAULT_NAMESPACE").unwrap();
    let mut client = VaultClient::new(
        VaultClientSettingsBuilder::default()
            .address(address)
            .set_namespace(namespace)
            .token("")
            .timeout(Some(core::time::Duration::from_secs(30)))
            .build()
            .unwrap(),
    )
    .unwrap();

    let token = match std::env::var("VAULT_TOKEN") {
        Ok(o) => {
            client.set_token(&o);
            match vaultrs::token::lookup_self(&client).await {
                Ok(lr) => {
                    if lr.ttl > 30 {
                        o
                    } else {
                        vault_auth(&client).await.client_token
                    }
                }
                Err(_) => vault_auth(&client).await.client_token,
            }
        }
        Err(_) => vault_auth(&client).await.client_token,
    };
    client.set_token(&token);
    std::env::set_var("VAULT_TOKEN", &token);
    client
}

pub async fn generate_pph(ident: &str) -> String {
    let mut password = [0u8; 1024];
    OsRng.fill_bytes(&mut password);
    let mut hasher = sha2::Sha512::new();
    hasher.update(password);
    let password = hex::encode(hasher.finalize());
    let mount = std::env::var("MOUNT").unwrap_or_else(|_| "secret".to_string());
    let mut path = std::env::var("VPATH").unwrap();
    let vault = vault_connect().await;
    //path.push('/');
    path.push_str(ident);
    let mut data = HashMap::<&str, &str>::new();
    data.insert("pw", &password);
    let _set = vaultrs::kv2::set(&vault, &mount, &path, &data)
        .await
        .unwrap();
    password
}

pub async fn encrypt_pvks(source: &[String], ident: &str) -> Result<Vec<String>, MurinError> {
    let password = generate_pph(ident).await;
    let mut ret = Vec::<String>::new();
    for s in source {
        ret.push(crate::encryption::encrypt(s, &password)?)
    }
    Ok(ret)
}

pub async fn encrypt_data(source: &String, ident: &str) -> Result<String, MurinError> {
    let password = generate_pph(ident).await;
    let cipher = encrypt(source, &password)?;
    Ok(cipher)
}

pub fn encrypt(source: &String, password: &String) -> Result<String, MurinError> {
    let mut nonce = [0u8; 19];
    OsRng.fill_bytes(&mut nonce);

    let (mut key, mut salt) = get_secure_key_from_pwd(password);

    let aead = XChaCha20Poly1305::new(key[..32].as_ref().into());
    let mut stream_encryptor = stream::EncryptorBE32::from_aead(aead, nonce.as_ref().into());

    let mut source_data = source.as_bytes();
    let mut dist = BufWriter::new(Vec::new());

    dist.write_all(&salt)?;
    dist.write_all(&nonce)?;
    const BUFFER_LEN: usize = 500;
    let mut buffer = [0u8; BUFFER_LEN];

    loop {
        let read_count = source_data.read(&mut buffer)?;

        if read_count == BUFFER_LEN {
            let ciphertext = stream_encryptor
                .encrypt_next(buffer.as_slice())
                .map_err(|err| murin::MurinError::new(&format!("Encrypting: {}", err)))?;
            dist.write_all(&ciphertext)?;
        } else {
            let ciphertext = stream_encryptor
                .encrypt_last(&buffer[..read_count])
                .map_err(|err| murin::MurinError::new(&format!("Encrypting: {}", err)))?;
            dist.write_all(&ciphertext)?;
            break;
        }
    }

    let bytes = dist.into_inner().unwrap();
    let string = hex::encode(bytes);

    salt.zeroize();
    nonce.zeroize();
    key.zeroize();

    Ok(string)
}

pub async fn decrypt_data(encrypted_source: &String, ident: &str) -> Result<String, MurinError> {
    let mount = std::env::var("MOUNT").unwrap_or_else(|_| "secret".to_string());
    let mut path = std::env::var("VPATH").unwrap();
    let vault = vault_connect().await;

    path.push_str(ident);
    let p: HashMap<String, String> = vaultrs::kv2::read(&vault, &mount, &path).await.unwrap();
    let clear = decrypt(encrypted_source, p.get("pw").unwrap())?;
    Ok(clear)
}

pub fn decrypt(encrypted_source: &String, password: &String) -> Result<String, MurinError> {
    let mut salt = [0u8; 32];
    let mut nonce = [0u8; 19];

    let mut encrypted_data = &hex::decode(encrypted_source).unwrap()[..];
    let mut dist = BufWriter::new(Vec::new());

    let mut read_count = encrypted_data.read(&mut salt)?;
    if read_count != salt.len() {
        return Err(murin::MurinError::new("Error reading salt."));
    }

    read_count = encrypted_data.read(&mut nonce)?;
    if read_count != nonce.len() {
        return Err(murin::MurinError::new("Error reading nonce."));
    }

    let argon2_config = argon2_config();
    let mut key = rargon2::hash_raw(password.as_bytes(), &salt, &argon2_config)
        .map_err(|err| murin::MurinError::new(&format!("{}", err)))?;

    let aead = XChaCha20Poly1305::new(key[..32].as_ref().into());
    let mut stream_decryptor = stream::DecryptorBE32::from_aead(aead, nonce.as_ref().into());

    const BUFFER_LEN: usize = 500 + 16;
    let mut buffer = [0u8; BUFFER_LEN];

    loop {
        let read_count = encrypted_data.read(&mut buffer)?;

        if read_count == BUFFER_LEN {
            let plaintext = stream_decryptor
                .decrypt_next(buffer.as_slice())
                .map_err(|err| murin::MurinError::new(&format!("Decrypting: {}", err)))?;
            dist.write_all(&plaintext)?;
        } else if read_count == 0 {
            break;
        } else {
            let plaintext = stream_decryptor
                .decrypt_last(&buffer[..read_count])
                .map_err(|err| murin::MurinError::new(&format!("Decrypting: {}", err)))?;
            dist.write_all(&plaintext)?;
            break;
        }
    }

    salt.zeroize();
    nonce.zeroize();
    key.zeroize();

    let bytes = dist
        .into_inner()
        .map_err(|err| murin::MurinError::new(&format!("{}", err)))?;
    let string = String::from_utf8(bytes)?;

    Ok(string)
}

pub async fn decrypt_pkvs(vec: Vec<String>, ident: &str) -> Result<Vec<String>, MurinError> {
    let mut epvks = Vec::<String>::new();
    for pv in vec {
        epvks.push(crate::encryption::decrypt_data(&pv, ident).await?)
    }
    Ok(epvks)
}

pub fn mident(u: &i64, ci: &i64, v: &f32, ca: &String) -> String {
    let mut hasher = sha2::Sha224::new();
    hasher.update((*u).to_ne_bytes());
    hasher.update((*ci).to_ne_bytes());
    hasher.update((*v).to_ne_bytes());
    hasher.update((*ca).as_bytes());
    hex::encode(hasher.finalize())
}
