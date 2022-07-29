# Drasil Application Framework

THe Drasil Application Framework is collection of microservices which act together as scalabe off chain framework for the Cardano blockchain. 
Its different applications and libraries are explained below.

Drasil was invented and formed by Torben and Zak but it uses several tools of the Cardano community and to run a full "Drasil System" it needs more than the applications and libraries you can find in this repository. Torben and Zak will help you to utillize this framework for your application and consult you on what it needs to run it if you want to, just contact us. 
Drasil is made for Orechstration can run in Kubernetes, managed or not, on native Linux or locally in your docker environment. We constantly develop on Drasil and 
work closely together with [Maestro](https://www.gomaestro.org) for infrastrucutre and managed Drasil solutions. 

* [Documentation](https://docs.drasil.io/)

* [License]( https://www.drasil.io/licenses/LICENSE-1.0)

## Mythology
The word "Drasil" derives from "Yggdrasil" which is in north mythology the world tree there are some different transalation and interpretations which you can find below. We chose Drasil as we this application framework are the branches of a large tree, the "World tree" which supports Cardanos applications on the blockchain.


[Wikipedia](https://sv.wikipedia.org/wiki/Vidar)


##

## Building Drasil

### Cargo

#### Prerequisites
* [Rust](https://www.rust-lang.org/tools/install/)

#### Build Executable

Navigate into the 'drasil' folder and run `cargo build`. This will build all services at once.
If you want to build executables for production use for example: 

`RUSTFLAGS='-C target-feature=-crt-static' cargo build --target x86_64-unknown-linux-gnu --release`

or

`RUSTFLAGS='-C target-feature=+crt-static' cargo build --target x86_64-unknown-linux-musl --release`


### Docker

#### Prerequisites
* [Rust](https://www.rust-lang.org/tools/install/)
* [Docker](https://docs.docker.com/engine/install/)
* [Docker-Compose](https://docs.docker.com/compose/install/)



#### Build image
```
make build
```
Will start building a docker image 

#### Run container
```
make run
```
Will start the image as local docker container 

#### Push image to Registry
```
make push
```
The docker image path and name can be defined in the Makefile using the following command will initiate a push. 


### Environment 
Drasils individual services need many settings which are passed via environment variables, the individual needed settings are described in the readme files for each service. 

## Architecture