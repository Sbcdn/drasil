#!/bin/bash
export PGPASSWORD=drasiluserpassword
while [ "$(kubectl get pods postgresql-db-rwd-0 -ojson | jq '.status.containerStatuses[].ready')" == "false" ]; do echo "wait for reward database to start up ..."; sleep 3; done; \
kubectl exec -it postgresql-db-rwd-0 -- psql -h localhost -U drasiluser -p 5432 -d rewarddb -a -f ./migrations/rewarddb_tables.sql
while [ "$(kubectl get pods postgresql-db-sys-0 -ojson | jq '.status.containerStatuses[].ready')" == "false" ]; do echo "wait for system database to start up ..."; sleep 3; done; \
kubectl exec -it postgresql-db-sys-0 -- psql -h localhost -U drasiluser -p 5432 -d systemdb -a -f ./migrations/systemdb_tables.sql