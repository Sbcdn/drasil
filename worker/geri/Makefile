REGISTRY=default-route-openshift-image-registry.apps.testnet.drasil.org
PROJECT=drasil
IMAGE=geri
VERSION=v1.0

build:
	cargo update
	cargo fetch
	rm -rf .cargo
	mkdir .cargo
	echo "git-fetch-with-cli = true" > .cargo/config
	cp -R ~/.cargo/git .cargo/git
	cp -R ../../libs .cargo/libs
	sudo docker build -t $(REGISTRY)/$(PROJECT)/$(IMAGE):$(VERSION) -f Dockerfile --target $(IMAGE) .

run:
	docker run -t $(IMAGE):$(VERSION) $(IMAGE)

push:
	sudo docker push $(REGISTRY)/$(PROJECT)/$(IMAGE):$(VERSION)
	
GOOGLEREG=europe-west2-docker.pkg.dev
PROJECT_GOOGLE=kebvmwbusajq
REPO_GOOGLE=prod-kube-repo

build-mainnet-google:
	cargo update
	cargo fetch
	rm -rf .cargo
	mkdir .cargo
	echo "git-fetch-with-cli = true" > .cargo/config
	cp -R ~/.cargo/git .cargo/git
	cp -R ../../libs .cargo/libs
	sudo docker build -t $(GOOGLEREG)/$(PROJECT_GOOGLE)/$(REPO_GOOGLE)/$(IMAGE):$(VERSION) -f Dockerfile --target $(IMAGE) .

push-mainnet-google:
	docker push $(GOOGLEREG)/$(PROJECT_GOOGLE)/$(REPO_GOOGLE)/$(IMAGE):$(VERSION)