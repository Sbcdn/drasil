# Drasil Local Setup
For Development and Test purposes a local kubernetes setup using k3d is included. 
For further information and troubleshooting go to: https://k3d.io/v5.0.1/
You will need to work with 'kubectl': https://kubernetes.io/docs/reference/kubectl/

Some Basics: 

`kubectl get pods` - List the pods

`kubectl logs <pod-name> <container-name>` - Pod logs Example: `kubectl logs odin-fdfdbd8d9-nnsl8 odin` or `kubectl logs odin-fdfdbd8d9-nnsl8 dvltath`. If only one container exists the `<container-name>` can be omitted.

`kubectl delete pod <podname>` - Deletes a pod, will be automatically restarted from the deployment (restart a pod)

`kubectl describe pod <podname>` - Get somre more information about a pod

There is a API documentation made for the local setup and maps to the correct ports already. 

## Precondition
- Install Docker: https://docs.docker.com/engine/install/ and setup permission for your user to use docker. 
- Install Cardano Node and Cardano DBsync for Testnet or have a possibility to connect to a dbsync

### Cardano Node and Dbsync
The easiest way to setup a cardano node is using [mithtil](https://mithril.network/doc/manual/getting-started/bootstrap-cardano-node). How to setup a cardano dbsync is described [here](https://github.com/IntersectMBO/cardano-db-sync/blob/master/doc/docker.md).


## Install dependencies

`make setup-dependencies`

Will install the dependencies, tested on Ubuntu 22.04, requires `sudo` !

Dependencies: 
    - curl
    - make
    - jq
    - kubectl
    - k3d
    - helm

## First Setup
If Docker is installed you can do: 

`make first-setup`

It will install everything from scratch, start a cluster, build and load all deployments.

## Start a Local Drasil Cluster

`make start-local-cluster`

This command will delete a current cluster if existing and setup a new one from scratch, it will not install and also not build the images.  
Note that it can take a moment until all pods are operational and they might crash until the cluster is fully ready (usually ~1 min).
You will realise this on the "geri" service which expects the "transactions" stream to exist which needs to be provided by Oura in the redis database, it might take a minute.

## Build new Docker Images

`make build-all-local`

Will trigger the rebuild of the docker images. Drasil uses a two step build process. If you made changes to the codebase you will need to run the "drasil-builder" first, which will build all libraries and binaries. The 'build-all-loc' all will just assemble the service images and tag them.

`make build-drasil` will do the whole sequence for you. 

## Push Images to Registry

`make push-all-local`

Push the latest build service images to the local registry.

## Deploy Deployments to Cluster

`make local-deploy`

Loads the kubernetes configs, deployments etc. into the cluster.

## Delete local cluster

`make delete-cluster`

Deletes the local cluster


## After starting the cluster

- Take a look at the kubernetes dashboard, the url and login token are written to the terminal at the end of `make start-local-cluster` (deletes existing clusters)
- Make sure you create an API token for your `dadmin` user via the REST API (frigg) and use it for authentication in requests to the TxBuilder's (heimdalr, loki,...) or Information Endpoints (vidar,..)
- To use the admin API you need to login and use the returned bearer token in requests, it expires after 60 minutes. Find the API documentation [here] (https://documenter.getpostman.com/view/23201834/2s9YXpUHwG)
- `make stop-local-cluster` stop cluster without deleting the local cluster
- `make restart-local-cluster` restatrs an existing local cluster

### Default User
User: `dadmin`
E-Mail: `dadmin@drasil.io`
Password: `drasil123`

### Ports
- TxBuilding API (heimdallr): localhost:30000
- Information API (vidar): localhost:30001
- Websocket for NFTs (loki): localhost:30002
- Admin Rest API (frigg): localhost:30003

API Docs: https://documenter.getpostman.com/view/23201834/2s9YXpUHwG

Use the generated Bearer Token for authentication at heimdallr, vidar and loki.

### Remarks on performance
The performance of request is mainly depending on the dbsync connection. As closer and faster the dbsync is, as faster is drasil.

