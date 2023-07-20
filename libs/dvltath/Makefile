REGISTRY=default-route-openshift-image-registry.apps.testnet.drasil.org
PROJECT=drasil
IMAGE=dvltath
VERSION=v1.0

build:
	cargo update
	cargo fetch
	rm -rf .cargo
	mkdir .cargo
	echo "git-fetch-with-cli = true" > .cargo/config
	cp -R ~/.cargo/git .cargo/git
	sudo docker build -t $(REGISTRY)/$(PROJECT)/$(IMAGE):$(VERSION) -f Dockerfile --target $(IMAGE) .

run:
	docker run -t $(REGISTRY)/$(PROJECT)/$(IMAGE):$(VERSION)

push:
	sudo docker push $(REGISTRY)/$(PROJECT)/$(IMAGE):$(VERSION)

GOOGLEREG=europe-west2-docker.pkg.dev
PROJECT_GOOGLE=tqjwvihfudnb
REPO_GOOGLE=wm-staging-registry

build-mainnet-google:
	cargo update
	cargo fetch
	rm -rf .cargo
	mkdir .cargo
	echo "git-fetch-with-cli = true" > .cargo/config
	cp -R ~/.cargo/git .cargo/git
	docker build -t $(GOOGLEREG)/$(PROJECT_GOOGLE)/$(REPO_GOOGLE)/$(IMAGE):$(VERSION) -f Dockerfile --target $(IMAGE) .

push-mainnet-google:
	docker push $(GOOGLEREG)/$(PROJECT_GOOGLE)/$(REPO_GOOGLE)/$(IMAGE):$(VERSION)