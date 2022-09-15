REGISTRY=default-route-openshift-image-registry.apps.testnet.drasil.org
PROJECT=mdrsl
IMAGE=dvltath
VERSION=v1.1

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

