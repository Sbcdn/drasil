REGION=europe-west2-docker.pkg.dev

MAINNET_REGISTRY=prod-kube-repo
MAINNET_PROJECT=kebvmwbusajq

TESTNET_REGISTRY=preview-testnet-registry
TESTNET_PROJECT=efvgtwmyqlpe

LOC_PROJECT=minikube
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

# Build for Minikube
build-all-loc:
	docker build --progress=plain -t $(LOC_PROJECT)/vidar:$(VERSION) -f Dockerfile --target=vidar .
	docker build --progress=plain -t $(LOC_PROJECT)/heimdallr:$(VERSION) -f Dockerfile --target=heimdallr .
	docker build --progress=plain -t $(LOC_PROJECT)/odin:$(VERSION) -f Dockerfile --target=odin .
	docker build --progress=plain -t $(LOC_PROJECT)/loki:$(VERSION) -f Dockerfile --target=loki .
	docker build --progress=plain -t $(LOC_PROJECT)/frigg:$(VERSION) -f Dockerfile --target=frigg .
	docker build --progress=plain -t $(LOC_PROJECT)/geri:$(VERSION) -f Dockerfile --target=geri .
	docker build --progress=plain -t $(LOC_PROJECT)/drasil-jobs:$(VERSION) -f Dockerfile --target=drasil_jobs .
	docker build --progress=plain -t $(LOC_PROJECT)/work-loki:$(VERSION) -f Dockerfile --target=work_loki .
	docker build --progress=plain -t $(LOC_PROJECT)/freki:$(VERSION) -f Dockerfile --target=freki .
	docker build --progress=plain -t $(LOC_PROJECT)/utxopti:$(VERSION) -f Dockerfile --target=utxopti .
	docker build --progress=plain -t $(LOC_PROJECT)/dvltath:$(VERSION) -f Dockerfile --target=dvltath .
	minikube image load $(LOC_PROJECT)/vidar:$(VERSION)
	minikube image load $(LOC_PROJECT)/heimdallr:$(VERSION)
	minikube image load $(LOC_PROJECT)/odin:$(VERSION)
	minikube image load $(LOC_PROJECT)/loki:$(VERSION)
	minikube image load $(LOC_PROJECT)/frigg:$(VERSION)
	minikube image load $(LOC_PROJECT)/geri:$(VERSION)
	minikube image load $(LOC_PROJECT)/drasil-jobs:$(VERSION)
	minikube image load $(LOC_PROJECT)/work-loki:$(VERSION)
	minikube image load $(LOC_PROJECT)/freki:$(VERSION)
	minikube image load $(LOC_PROJECT)/utxopti:$(VERSION)
	minikube image load $(LOC_PROJECT)/dvltath:$(VERSION)
	minikube cache reload
	minikube image ls