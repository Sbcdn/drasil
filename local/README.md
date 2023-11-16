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
You will release this on the "geri" service which expects a certain key exists in the redis database, oura will create those keys automatically but it might take a moment. As soon the keys are existing "geri" is expected to run smoothly.

## Build new Docker Images

`make build-all-local`

Will trigger the rebuild of the docker images. Drasil uses a two step build process. If you made changes to the codebase you will need to run the "drasil-builder" first, which will build all libraries and binaries. The 'build-all-loc' all will just assemble the service images and tag them.

`make build-drasil` will do both sequentially. 

## Push Images to Registry

`make push-all-local`

Push all the service images to the local registry.

## Deploy Deployments to Cluster

`make local-deploy`

Loads the kubernetes configs, deployments etc. into the cluster.

## Delete local cluster

`make delete-cluster`

Deletes the local cluster


## After starting the cluster

- Take a look at the kubernetes dashboard, the url and login token are written to the terminal at the end of `make start-local-cluster` (deletes existing clusters)
- Make sure you create an API token for your `dadmin` user via the REST API and use it for requests to the TxBuilder or Information Endpoints
- To use the admin API you need to login and use the returned bearer token in requests, it expires after 60 minutes.
- `make stop-local-cluster` stop cluster without deleting the local cluster
- `make restart-local-cluster` restatrs an existing local cluster

### Default User
User: `dadmin`
E-Mail: `dadmin@drasil.io`
Password: `drasiluserpassword`

### Ports
- Admin Rest API (frigg): localhost:30003
- TxBuilding API (heimdallr): localhost:30000
- Information API (vidar): localhost:30002

API Docs: https://documenter.getpostman.com/view/23201834/2s9YXpUHwG

### Remarks on performance
The performance of request is mainly depending on the dbsync connection. As closer and faster the dbsync is as faster is drasil.

