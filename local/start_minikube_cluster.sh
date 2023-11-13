#/bin/bash
minikube start
minikube addons enable metrics-server
nohup minikube dashboard &
nohup minikube mount $HOME/minikube_storage:minikube_storage &
minikube cache reload
kubectl apply -f ./configmaps/drasil_configmap.yaml
kubectl apply -f ./configmaps/dvltath_configmap.yaml
kubectl apply -f ./configmaps/frigg_configmap.yaml
kubectl apply -f ./configmaps/geri_configmap.yaml
kubectl apply -f ./configmaps/odin_configmap.yaml
kubectl apply -f ./configmaps/oura_configmap.yaml
# Postgres Database StatefulSet
#kubectl apply -f ./deployments/deplo_postgres.yaml
# Deployments
kubectl apply -f ./deployments/persistent_volume.yaml
kubectl apply -f ./deployments/deplo_odin.yaml
kubectl apply -f ./deployments/deplo_frigg.yaml
kubectl apply -f ./deployments/deplo_geri.yaml
kubectl apply -f ./deployments/deplo_heimdallr.yaml
kubectl apply -f ./deployments/deplo_loki.yaml
#kubectl apply -f ./deployments/deplo_oura.yaml
#kubectl apply -f ./deployments/deplo_redis.yaml
#kubectl apply -f ./deployments/deplo_rmq.yaml
#kubectl apply -f ./deployments/deplo_vault.yaml
kubectl apply -f ./deployments/deplo_vidar.yaml
kubectl apply -f ./deployments/deplo_worker_loki.yaml

