REGISTRY=default-route-openshift-image-registry.apps.testnet.drasil.org
PROJECT=mdrsl
IMAGE=frigg
VERSION=v1.1

build:
	cargo update
	cargo fetch
	rm -rf .cargo
	mkdir .cargo
	echo "git-fetch-with-cli = true" > .cargo/config
	cp -R ~/.cargo/git .cargo/git
	cp -R ../hugin .cargo/hugin
	cp -R ../murin .cargo/murin
	cp -R ../mimir .cargo/mimir
	cp -R ../gungnir .cargo/gungnir
	cp -R ../sleipnir .cargo/sleipnir
	cp -R ../dvltath .cargo/dvltath
	sudo docker build -t $(REGISTRY)/$(PROJECT)/$(IMAGE):$(VERSION) -f Dockerfile --target $(IMAGE) .

run:
	docker run -t $(IMAGE):$(VERSION) $(IMAGE)

push:
	sudo docker push $(REGISTRY)/$(PROJECT)/$(IMAGE):$(VERSION)
