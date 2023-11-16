#!/bin/bash
while [ "$(kubectl get pods vault-0 -ojson | jq '.status.containerStatuses[].ready')" == "false" ]; do sleep 3; done; \
kubectl cp ./local/scripts/configure.sh default/vault-0:/tmp/ -c vault; \
kubectl get pods vault-0 -ojson | jq '.status.containerStatuses[].ready'; \
kubectl exec -i "vault-0" -- chmod +x /tmp/configure.sh; \
kubectl exec -i "vault-0" -- /tmp/configure.sh