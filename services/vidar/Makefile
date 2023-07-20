REGISTRY=default-route-openshift-image-registry.apps.testnet.drasil.org
PROJECT=drasil
MAINNET_PROJECT=mdrsl
IMAGE=vidar
VERSION=v1.3

build:
	cargo update
	cargo fetch
	rm -rf .cargo
	mkdir .cargo
	echo "git-fetch-with-cli = true" > .cargo/config
	cp -R ~/.cargo/git .cargo/git
	cp -R ../../libs .cargo/libs
	sudo docker build -t $(REGISTRY)/$(PROJECT)/$(IMAGE):$(VERSION) -f Dockerfile --target $(IMAGE) .

build-mainnet:
	cargo update
	cargo fetch
	rm -rf .cargo
	mkdir .cargo
	echo "git-fetch-with-cli = true" > .cargo/config
	cp -R ~/.cargo/git .cargo/git
	cp -R ../../libs .cargo/libs
	sudo docker build -t $(REGISTRY)/$(MAINNET_PROJECT)/$(IMAGE):$(VERSION) -f Dockerfile --target $(IMAGE) .

run:
	docker run -t $(IMAGE):$(VERSION) $(IMAGE)

push:
	sudo docker push $(REGISTRY)/$(PROJECT)/$(IMAGE):$(VERSION)

push-mainnet:
	sudo docker push $(REGISTRY)/$(MAINNET_PROJECT)/$(IMAGE):$(VERSION)

#GOOGLEREG=europe-west2-docker.pkg.dev
#PROJECT_GOOGLE=mystic-torus-339723
#REPO_GOOGLE=drasil-mainnet

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