#/bin/bash
nohup minikube start &
nohup minikube dashboard &
nohup minikube mount $HOME/minikube_storage:minikube_storage &
minikube cache reload &
kubectl apply -f ./deployments/persistent_volume.yaml
kubectl apply -f ./deployments/deplo_odin.yaml
kubectl apply -f ./deployments/deplo_frigg.yaml

