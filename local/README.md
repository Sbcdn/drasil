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

## First Setup
If Docker is installed you can do: 

`make first-setup`

It will install kubectl, helm and k3d and then build the docker images.
Afterwards it will setup the local cluster and do the deployments. 

This step is only needed the first time.

## Start a Local Drasil Cluster

`make setup-local-all`

This command will delete a current cluster if existing and setup a new one from scratch, it will not install and also not build the images.  
Note that it can take a moment until all pods are operational and they might crash until the cluster is fully ready (usually ~1 min).
You will release this on the "geri" service which expects a certain key exists in the redis database, oura will create those keys automatically but it might take a moment. As soon the keys are existing "geri" is expected to run smoothly.

## Build new Docker Images

`make build-all-loc`

Will trigger the rebuild of the docker images. Drasil uses a two step build process. If you made changes to the codebase you will need to run the "drasil-builder" first (slow), which will build all libraries and binaries. The 'build-all-loc' all will just assemble the service images and tag them (quick).

## Push Images to Registry

`make push-all-loc`

Push all the service images to the local registry.

## Deploy Deployments to Cluster

`make local-deploy`

Loads the kubernetes configs, deployments etc. into the cluster.

## Delete local cluster

`make delete-cluster`

Deletes the local cluster


# After starting the cluster

- Make sure you call `make setup-databases` after starting a new cluster to create the database tables and the default user. This step is not automated yet. 
- Take a look at the kubernetes dashboard, the url and login token are written to the terminal on `make start-local-cluster` 
- Make sure you create an API token for your `dadmin` user and use it for requests to the TxBuilder or Information Endpoints
- To use the admin API you need to login and use the login token in the requests, it expires after 60 minutes.

## Default User
User: `dadmin`
E-Mail: `dadmin@drasil.io`
Password: `***REMOVED***password`

## Ports
- Admin Rest API (frigg): localhost:30003
- TxBuilding API (heimdallr): localhost:30000
- Information API (vidar): localhost:30002

## Remarks on performance
The performance of request is mainly depending on the dbsync connection. As closer and faster the dbsync is as faster is drasil.

