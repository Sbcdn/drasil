#!/bin/sh
export VAULT_TOKEN="root"
export VAULT_ADDR=http://127.0.0.1:8200
export VAULT_DISABLE_USER_LOCKOUT=true
vault auth enable approle
vault policy write pdrslapp - << EOF

path "secret/data/drasil/*" {
    capabilities = ["create", "read", "list", "update", "patch"]
}

path "secret/drasil/*" {
    capabilities = ["create", "read", "list", "update", "patch"]
}

path "auth/token/lookup-self" {
    capabilities = ["read"]
}

path "auth/approle/login" {
    capabilities = ["create"]
}
EOF

vault write auth/approle/role/drslapp \
        secret_id_ttl=10s \
        secret_id_num_uses=1 \
        enable_local_secret_ids=false \
        token_num_uses=0 \
        token_ttl=15m \
        token_max_ttl=1h \
        token_type=default \
        token_renewable=true \
        period="" \
        policies="default","pdrslapp"


vault policy write pmngrdrslapp - << EOF
path "auth/approle/role/drslapp/secret-id" {
    capabilities = ["create", "update"]
    min_wrapping_ttl = "1s"
    max_wrapping_ttl = "60s"
}
path "auth/approle/login" {
    capabilities = ["create",]
}
path "auth/token/lookup-self" {
    capabilities = [ "read"]
}
EOF
        
vault write auth/approle/role/mngdrslapp \
        secret_id_ttl=0 \
        secret_id_num_uses=0 \
        enable_local_secret_ids=false \
        token_num_uses=0 \
        token_ttl=0 \
        token_max_ttl=0 \
        token_type=default \
        period="" \
        policies="default","pmngrdrslapp"