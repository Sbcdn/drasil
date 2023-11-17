#!/bin/bash
while [ "$(kubectl get pods vault-0 -ojson | jq '.status.containerStatuses[].ready')" == "false" ]; do sleep 3; done; \
kubectl cp ./local/scripts/configure.sh default/vault-0:/tmp/ -c vault; \
kubectl get pods vault-0 -ojson | jq '.status.containerStatuses[].ready'; \
kubectl exec -i "vault-0" -- chmod +x /tmp/configure.sh; \
kubectl exec -i "vault-0" -- /tmp/configure.sh
APP_ROLE_ID=$(kubectl exec -i vault-0 -- vault read -format=json auth/approle/role/drslapp/role-id | jq -r '.data.role_id')
kubectl create secret generic app-vault \
    --from-literal=role-id=${APP_ROLE_ID} 
DVLTATH_ROLE_ID=$(kubectl exec -i vault-0 -- vault read -format=json auth/approle/role/mngdrslapp/role-id | jq -r '.data.role_id')
DVLTATH_SECRET_ID=$(kubectl exec -i vault-0 -- vault write -f -format=json auth/approle/role/mngdrslapp/secret-id | jq -r '.data.secret_id')
echo $DVLTATH_ROLE_ID
echo $DVLTATH_SECRET_ID
kubectl create secret generic dvltath-vault \
    --from-literal=role-id=${DVLTATH_ROLE_ID} \
    --from-literal=secret-id=${DVLTATH_SECRET_ID} 