#!/bin/sh
vault policy write drslapp1 - << EOF

path "secret/data/drasil/*" {
  capabilities = ["create", "read", "list", "update", "patch"]
}

path "secret/drasil/*" {
  capabilities = ["create", "read", "list", "update", "patch"]
}

path "auth/token/lookup-self" {
  capabilities = ["read"]
}
EOF

# This effectively makes response wrapping mandatory for this path by setting min_wrapping_ttl to 1 second.
# This also sets this path's wrapped response maximum allowed TTL to 90 seconds.
vault policy write mngrdrslapp1 - << EOF
path "auth/approle/role/drslapp/secret-id" {
    capabilities = ["create", "update"]
    min_wrapping_ttl = "1s"
    max_wrapping_ttl = "90s"
}
EOF

vault auth enable approle

vault write auth/approle/role/drslapp \
      secret_id_bound_cidrs="10.100.128.0/17","10.0.128.0/19","10.0.160.0/19" \
      secret_id_ttl=10s \
      secret_id_num_uses=1 \
      enable_local_secret_ids=false \
      token_bound_cidrs="10.100.128.0/17","10.0.128.0/19","10.0.160.0/19" \
      token_num_uses=0 \
      token_ttl=15m \
      token_max_ttl=1h \
      token_type=default \
      token_renewable=true \
      period="" \
      policies="default","drslapp1"

      
vault write auth/approle/role/mngdrslapp \
      secret_id_bound_cidrs="10.100.128.0/17","10.0.128.0/19","10.0.160.0/19" \
      secret_id_ttl=0 \
      secret_id_num_uses=0 \
      enable_local_secret_ids=false \
      token_bound_cidrs="10.100.128.0/17","10.0.128.0/19","10.0.160.0/19" \
      token_num_uses=0 \
      token_ttl=0 \
      token_max_ttl=0 \
      token_type=default \
      period="" \
      policies="default","mngrdrslapp1"