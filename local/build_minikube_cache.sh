#/bin/bash
eval $(minikube docker-env)
if [ $MINIKUBE_ACTIVE_DOCKERD == 'minikube' ]; then
	cd ..
	make build-drasil-builder
	make build-all-loc
fi

