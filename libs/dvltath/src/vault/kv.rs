use super::auth::*;
use std::collections::HashMap;

pub async fn vault_store(ident: &str, key: &str, value: &str) {
    let mut path = get_v_path();
    let vault = vault_connect().await;
    path.push_str(ident);
    let mut data = HashMap::<&str, &str>::new();
    data.insert(key, value);
    let _set = vaultrs::kv2::set(&vault, &get_gl_mount(), &path, &data)
        .await
        .unwrap();
}

pub async fn vault_get(ident: &str) -> HashMap<String, String> {
    let vault = vault_connect().await;
    let mut path = get_v_path();
    path.push_str(ident);
    vaultrs::kv2::read(&vault, &get_gl_mount(), &path)
        .await
        .unwrap()
}
