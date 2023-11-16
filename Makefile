REGION=europe-west2-docker.pkg.dev

MAINNET_REGISTRY=prod-kube-repo
MAINNET_PROJECT=kebvmwbusajq

TESTNET_REGISTRY=preview-testnet-registry
TESTNET_PROJECT=efvgtwmyqlpe

LOC_PROJECT=k3d-drasil-registry.localhost:12345
VERSION=v1.3

# Create Docker Compose for Local Setup
run:
	docker run -t $(IMAGE):$(VERSION) $(IMAGE)

build-drasil-builder:
	cargo update
	cargo fetch
	rm -rf .cargo
	mkdir .cargo
	mkdir .cargo/workspace
	echo "git-fetch-with-cli = true" > .cargo/config
	cp -R ~/.cargo/git .cargo/git
	cp -R ./drasil-dvltath .cargo/workspace/
	cp -R ./drasil-gungnir .cargo/workspace/
	cp -R ./drasil-hugin .cargo/workspace/
	cp -R ./drasil-mimir .cargo/workspace/
	cp -R ./drasil-murin .cargo/workspace/
	cp -R ./drasil-sleipnir .cargo/workspace/
	cp -R ./protocol_parameters.json .cargo/workspace/
	cp -R ./jobs .cargo/workspace/
	cp -R ./services .cargo/workspace/
	cp -R ./worker .cargo/workspace/
	cp -R ./Cargo.toml .cargo/workspace/
	docker build -t drasil/builder:latest --progress=plain -f Dockerfile --target=drasil-builder .

# Build Mainnet
build-vidar-mainnet:
	docker build --progress=plain -t $(REGION)/$(MAINNET_PROJECT)/$(MAINNET_REGISTRY)/vidar:$(VERSION) -f Dockerfile --target=vidar .	

build-heimdallr-mainnet:
	docker build --progress=plain -t $(REGION)/$(MAINNET_PROJECT)/$(MAINNET_REGISTRY)/heimdallr:$(VERSION) -f Dockerfile --target=heimdallr .	

build-odin-mainnet:
	docker build --progress=plain -t $(REGION)/$(MAINNET_PROJECT)/$(MAINNET_REGISTRY)/odin:$(VERSION) -f Dockerfile --target=odin .	


build-all-mainnet:
	make build-vidar-testnet
	make build-heimdallr-testnet
	make build-odin-testnet


# Push Mainnet
push-vidar-mainnet:
	docker push $(REGION)/$(MAINNET_PROJECT)/$(MAINNET_REGISTRY)/vidar:$(VERSION)

push-heimdallr-mainnet:
	docker push $(REGION)/$(MAINNET_PROJECT)/$(MAINNET_REGISTRY)/heimdallr:$(VERSION)

push-odin-mainnet:
	docker push $(REGION)/$(MAINNET_PROJECT)/$(MAINNET_REGISTRY)/odin:$(VERSION)


push-all-mainnet:
	make push-vidar-testnet
	make push-heimdallr-testnet
	make push-odin-testnet


# Build Testnet
build-vidar-testnet:
	docker build --progress=plain -t $(REGION)/$(TESTNET_PROJECT)/$(TESTNET_REGISTRY)/vidar:$(VERSION) -f Dockerfile --target=vidar .	

build-heimdallr-testnet:
	docker build --progress=plain -t $(REGION)/$(TESTNET_PROJECT)/$(TESTNET_REGISTRY)/heimdallr:$(VERSION) -f Dockerfile --target=heimdallr .	

build-odin-testnet:
	docker build --progress=plain -t $(REGION)/$(TESTNET_PROJECT)/$(TESTNET_REGISTRY)/odin:$(VERSION) -f Dockerfile --target=odin .	

build-loki-testnet:
	docker build --progress=plain -t $(REGION)/$(TESTNET_PROJECT)/$(TESTNET_REGISTRY)/loki:$(VERSION) -f Dockerfile --target=loki .	

build-frigg-testnet:
	docker build --progress=plain -t $(REGION)/$(TESTNET_PROJECT)/$(TESTNET_REGISTRY)/frigg:$(VERSION) -f Dockerfile --target=frigg .	

build-geri-testnet:
	docker build --progress=plain -t $(REGION)/$(TESTNET_PROJECT)/$(TESTNET_REGISTRY)/geri:$(VERSION) -f Dockerfile --target=geri .	

build-drasil-jobs-testnet:
	docker build --progress=plain -t $(REGION)/$(TESTNET_PROJECT)/$(TESTNET_REGISTRY)/drasil-jobs:$(VERSION) -f Dockerfile --target=drasil_jobs .	

build-work-loki-testnet:
	docker build --progress=plain -t $(REGION)/$(TESTNET_PROJECT)/$(TESTNET_REGISTRY)/work-loki:$(VERSION) -f Dockerfile --target=work_loki .	

build-freki-testnet:
	docker build --progress=plain -t $(REGION)/$(TESTNET_PROJECT)/$(TESTNET_REGISTRY)/freki:$(VERSION) -f Dockerfile --target=freki .	

build-utxopti-testnet:
	docker build --progress=plain -t $(REGION)/$(TESTNET_PROJECT)/$(TESTNET_REGISTRY)/utxopti:$(VERSION) -f Dockerfile --target=utxopti .

build-dvltath-testnet:
	docker build --progress=plain -t $(REGION)/$(TESTNET_PROJECT)/$(TESTNET_REGISTRY)/dvltath:$(VERSION) -f Dockerfile --target=dvltath .

build-all-testnet:
	make build-vidar-testnet
	make build-heimdallr-testnet
	make build-odin-testnet
	make build-loki-testnet
	make build-frigg-testnet
	make build-geri-testnet
	make build-drasil-jobs-testnet
	make build-work-loki-testnet
	make build-freki-testnet
	make build-utxopti-testnet
	make build-dvltath-testnet


# Push Testnet
push-vidar-testnet:
	docker push $(REGION)/$(TESTNET_PROJECT)/$(TESTNET_REGISTRY)/vidar:$(VERSION)

push-heimdallr-testnet:
	docker push $(REGION)/$(TESTNET_PROJECT)/$(TESTNET_REGISTRY)/heimdallr:$(VERSION)

push-odin-testnet:
	docker push $(REGION)/$(TESTNET_PROJECT)/$(TESTNET_REGISTRY)/odin:$(VERSION)

push-loki-testnet:
	docker push $(REGION)/$(TESTNET_PROJECT)/$(TESTNET_REGISTRY)/loki:$(VERSION)

push-frigg-testnet:
	docker push $(REGION)/$(TESTNET_PROJECT)/$(TESTNET_REGISTRY)/frigg:$(VERSION)

push-geri-testnet:
	docker push $(REGION)/$(TESTNET_PROJECT)/$(TESTNET_REGISTRY)/geri:$(VERSION)

push-drasil-jobs-testnet:
	docker push $(REGION)/$(TESTNET_PROJECT)/$(TESTNET_REGISTRY)/drasil-jobs:$(VERSION)

push-work-loki-testnet:
	docker push $(REGION)/$(TESTNET_PROJECT)/$(TESTNET_REGISTRY)/work-loki:$(VERSION)

push-freki-testnet:
	docker push $(REGION)/$(TESTNET_PROJECT)/$(TESTNET_REGISTRY)/freki:$(VERSION)

push-utxopti-testnet:
	docker push $(REGION)/$(TESTNET_PROJECT)/$(TESTNET_REGISTRY)/utxopti:$(VERSION)

push-dvltath-testnet:
	docker push $(REGION)/$(TESTNET_PROJECT)/$(TESTNET_REGISTRY)/dvltath:$(VERSION)

push-all-testnet:
	make push-vidar-testnet
	make push-heimdallr-testnet
	make push-odin-testnet
	make push-loki-testnet
	make push-frigg-testnet
	make push-geri-testnet
	make push-drasil-jobs-testnet
	make push-work-loki-testnet
	make push-freki-testnet
	make push-utxopti-testnet
	make push-dvltath-testnet

# Build for Local Testing

# Docker must be installed already
setup-dependencies:
	sudo apt-get update
	sudo apt install -y curl
	curl https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3 | bash
	curl -s https://raw.githubusercontent.com/k3d-io/k3d/main/install.sh | bash
	sudo apt-get install -y apt-transport-https ca-certificates curl
	curl -fsSL https://pkgs.k8s.io/core:/stable:/v1.28/deb/Release.key | sudo gpg --dearmor -o /etc/apt/keyrings/kubernetes-apt-keyring.gpg
	echo 'deb [signed-by=/etc/apt/keyrings/kubernetes-apt-keyring.gpg] https://pkgs.k8s.io/core:/stable:/v1.28/deb/ /' | sudo tee /etc/apt/sources.list.d/kubernetes.list
	sudo apt-get update
	sudo apt-get install -y kubectl

create-local-cluster:
	@mkdir -p ${HOME}/k3dvol
	@mkdir -p ${HOME}/k3dvol/task-pv-volume-1
	@mkdir -p ${HOME}/k3dvol/task-pv-volume-2
	@mkdir -p ${HOME}/k3dvol/task-pv-volume-3
	@echo "Deleting old clusters..."
	@k3d cluster delete --all
	@echo "Creating new cluster..."
	@k3d cluster create --config ./local/local.yaml --api-port 6550 --registry-create k3d-drasil-registry.localhost:12345 --volume ${HOME}/k3dvol:${HOME}/k3dvol -p "30000-30010:30000-30010@server:0" --agents 2

build-all-loc:
# docker image delete drasil/builder:latest
# make build-drasil-builder
	@echo "Building local drasil images..."
	@docker build --progress=plain -t $(LOC_PROJECT)/vidar:$(VERSION) -f Dockerfile --target=vidar .
	@docker build --progress=plain -t $(LOC_PROJECT)/heimdallr:$(VERSION) -f Dockerfile --target=heimdallr .
	@docker build --progress=plain -t $(LOC_PROJECT)/odin:$(VERSION) -f Dockerfile --target=odin .
	@docker build --progress=plain -t $(LOC_PROJECT)/loki:$(VERSION) -f Dockerfile --target=loki .
	@docker build --progress=plain -t $(LOC_PROJECT)/frigg:$(VERSION) -f Dockerfile --target=frigg .
	@docker build --progress=plain -t $(LOC_PROJECT)/geri:$(VERSION) -f Dockerfile --target=geri .
	@docker build --progress=plain -t $(LOC_PROJECT)/drasil-jobs:$(VERSION) -f Dockerfile --target=drasil_jobs .
	@docker build --progress=plain -t $(LOC_PROJECT)/work-loki:$(VERSION) -f Dockerfile --target=work_loki .
	@docker build --progress=plain -t $(LOC_PROJECT)/freki:$(VERSION) -f Dockerfile --target=freki .
	@docker build --progress=plain -t $(LOC_PROJECT)/utxopti:$(VERSION) -f Dockerfile --target=utxopti .
	@docker build --progress=plain -t $(LOC_PROJECT)/dvltath:$(VERSION) -f Dockerfile --target=dvltath .


push-all-loc:
	@echo "Pushing to local registry..."
	@echo "Pushing vidar..."
	@docker push $(LOC_PROJECT)/vidar:$(VERSION)
	@echo "Pushing heimdallr..."
	@docker push $(LOC_PROJECT)/heimdallr:$(VERSION)
	@echo "Pushing odin..."
	@docker push $(LOC_PROJECT)/odin:$(VERSION)
	@echo "Pushing loki..."
	@docker push $(LOC_PROJECT)/loki:$(VERSION)
	@echo "Pushing frigg..."
	@docker push $(LOC_PROJECT)/frigg:$(VERSION)
	@echo "Pushing geri..."
	@docker push $(LOC_PROJECT)/geri:$(VERSION)
	@echo "Pushing drasil-jobs..."
	@docker push $(LOC_PROJECT)/drasil-jobs:$(VERSION)
	@echo "Pushing work-loki..."
	@docker push $(LOC_PROJECT)/work-loki:$(VERSION)
	@echo "Pushing freki..."
	@docker push $(LOC_PROJECT)/freki:$(VERSION)
	@echo "Pushing utxopti..."
	@docker push $(LOC_PROJECT)/utxopti:$(VERSION)
	@echo "Pushing dvltath..."
	@docker push $(LOC_PROJECT)/dvltath:$(VERSION)
	@echo "Done pushing to local registry."

local-deploy: 
	@echo "Applying Configmaps..."
	@kubectl apply -f ./local/configmaps/drasil_configmap.yaml
	@kubectl apply -f ./local/configmaps/dvltath_configmap.yaml
	@kubectl apply -f ./local/configmaps/frigg_configmap.yaml
	@kubectl apply -f ./local/configmaps/geri_configmap.yaml
	@kubectl apply -f ./local/configmaps/odin_configmap.yaml
	@kubectl apply -f ./local/configmaps/oura_configmap.yaml
	@kubectl apply -f ./local/configmaps/system_db.yaml
	@kubectl apply -f ./local/configmaps/reward_db.yaml
	@echo "Applying Deployments..."
# Tooling
	@kubectl apply -f ./local/deployments/deplo_oura.yaml
	@kubectl apply -f ./local/deployments/deplo_redis.yaml
	@kubectl apply -f ./local/deployments/deplo_rmq.yaml
# Deployments
	@kubectl apply -f ./local/deployments/persistent_volume.yaml
	@kubectl apply -f ./local/deployments/deplo_odin.yaml
	@kubectl apply -f ./local/deployments/deplo_frigg.yaml
	@kubectl apply -f ./local/deployments/deplo_geri.yaml
	@kubectl apply -f ./local/deployments/deplo_heimdallr.yaml
	@kubectl apply -f ./local/deployments/deplo_loki.yaml
	@kubectl apply -f ./local/deployments/deplo_vidar.yaml
	@kubectl apply -f ./local/deployments/deplo_worker_loki.yaml
	@kubectl apply -f ./local/deployments/deplo_drasil_job_processor.yaml
# Local Hashicorp Vault 
	@echo "Install Hashicorp Vault with helm..."
	@helm repo add hashicorp https://helm.rCaeleases.hashicorp.com | true
	@helm install vault hashicorp/vault --namespace default --set "server.dev.enabled=true" --set "server.dev.devRootToken=root" --set "server.global.tlsDisable=true" | true
# Stateful Sets
# Postgres Database StatefulSet
	@echo "Applying StatefulSets..."
	@kubectl apply -f ./local/deployments/deplo_postgres_system.yaml
	@kubectl apply -f ./local/deployments/deplo_postgres_reward.yaml
	@kubectl apply -f https://raw.githubusercontent.com/kubernetes/dashboard/v2.7.0/aio/deploy/recommended.yaml
	@chmod +x ./local/scripts/setup_vault.sh
	@./local/scripts/setup_vault.sh
#sh -c 'tar cf - ./local/scripts/configure.sh | kubectl exec -i vault-0 -- tar xf - -C /tmp/'
# Build for Local Testing
start-local-cluster:
	@echo "Starting New Cluster..."
	@-pkill -9 -f 'kubectl proxy' || true
	@make create-local-cluster
	@make push-all-loc
	@make local-deploy
	@nohup kubectl proxy &
	kubectl get pods
	@kubectl apply -f ./local/accounts/kubeadmin.yaml
	@echo "\nKubeadmin Token:\n"
	@kubectl get secret admin-user -n kubernetes-dashboard -o jsonpath={".data.token"} | base64 -d
	@echo "\n\nKubernetes Dashboard at:\nhttp://localhost:8001/api/v1/namespaces/kubernetes-dashboard/services/https:kubernetes-dashboard:/proxy/"
	@echo "\nSetup databases..."
	@chmod +x ./local/scripts/setup_database.sh
	@./local/scripts/setup_database.sh
	

delete-cluster:
	k3d cluster delete --all
	

first-setup: 
	make setup-dependencies
	make build-drasil-builder
	make build-all-loc
	make setup-local-all