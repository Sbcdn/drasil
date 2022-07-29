# Drasil Blockchain Application Framework - Frigg

Frigg provides an Admnistrative API to operate a client portal to configure and manage client applications and contracts.

See the API documentation under https://docs.drasil.io

## Prerequisites
* [Rust](https://www.rust-lang.org/tools/install/)
* [Docker](https://docs.docker.com/engine/install/)

Configure Google Cloud Platform access:
```
make configure-gcp
```

## Update cargo packages
```
make update
```

## Build image
```
make build
```

## Run container
```
make run
```

## Push image to Artifact Registry
```
make push
```
