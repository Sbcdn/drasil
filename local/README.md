# Drasil Local Setup

## Precondition
- Install Docker: https://docs.docker.com/engine/install/
- Install Cardano Node and Cardano DBsync for Testnet

## First Setup
When Docker is installed you can do: 

`make first-setup`

It will install kubectl, helm and k3d and then build the docker images.
Afterwards it will setup the local cluster and do the deployments. 

This step is only needed the first time.

## Start a Local Drasil Cluster

`make setup-local-all`

This command will delete a current cluster if existing and setup a new one from scratch. 
Expected that everything is installed and ready to use. 
Note that it can take a moment until all pods are operational and they might crash until the cluster is fully ready (usually ~1 min). 

## Build new Docker Images

`make build-all-loc`


## Push Images to Registry

`make push-all-loc`


## Deploy Deployments to Cluster

`make local-deploy`

## Delete local cluster

`make delete-cluster`

The local cluster is created with k3d, for further information and troubleshooting go to:

https://k3d.io/v5.0.1/