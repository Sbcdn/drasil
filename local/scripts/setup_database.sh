#!/bin/bash
export PGPASSWORD=drasiluserpassword
while [ "$(kubectl get pods postgresql-db-rwd-0 -ojson | jq '.status.containerStatuses[].ready')" == "false" ]; do sleep 1; done; \
kubectl exec -it postgresql-db-rwd-0 -- psql -h localhost -U drasiluser -p 5432 -d rewarddb -a -f ./migrations/rewarddb_tables.sql
while [ "$(kubectl get pods postgresql-db-sys-0 -ojson | jq '.status.containerStatuses[].ready')" == "false" ]; do sleep 1; done; \
kubectl exec -it postgresql-db-sys-0 -- psql -h localhost -U drasiluser -p 5432 -d systemdb -a -f ./migrations/systemdb_tables.sql
echo "\nDatabase tables and admin user created; User: dadmin, Password: drasil123; you can login now via admin service"