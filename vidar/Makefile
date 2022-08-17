REGISTRY=drasil-docker
PROJECT=drasil
REPOSITORY=drasil-blockchain-dapp-framework
IMAGE=vidar
VERSION=0.1

build:
	cargo update
	cargo fetch
	rm -rf .cargo
	mkdir .cargo
	echo "git-fetch-with-cli = true" > .cargo/config
	cp -R ~/.cargo/git .cargo/git
	cp -R ../gungnir .cargo/gungnir
	cp -R ../hugin .cargo/hugin
	cp -R ../murin .cargo/murin
	cp -R ../mimir .cargo/mimir
	docker build -t $(REGISTRY)/$(PROJECT)/$(REPOSITORY)/$(IMAGE):$(VERSION) -f Dockerfile --target $(IMAGE) .

run:
	docker run -t $(IMAGE):$(VERSION) $(IMAGE)

push:
	docker push $(REGISTRY)/$(PROJECT)/$(REPOSITORY)/$(IMAGE):$(VERSION)
