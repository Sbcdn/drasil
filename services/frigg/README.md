# Frigg

## Summary
Frigg provides an API to operate a client portal to configure and manage client applications and contracts.

* [Documentation](https://docs.drasil.io/reward-and-distribution/drasil-rewards-api/reward-information)

## Mythology
Frigg (/frɪɡ/; Old Norse: [ˈfriɡː])[1] is a goddess, one of the Æsir, in Germanic mythology. In Norse mythology, the source of most surviving information about her, she is associated with marriage, prophecy, clairvoyance and motherhood, and dwells in the wetland halls of Fensalir. In wider Germanic mythology, she is known in Old High German as Frīja, in Langobardic as Frēa, in Old English as Frīg, in Old Frisian as Frīa, and in Old Saxon as Frī, all ultimately stemming from the Proto-Germanic theonym *Frijjō, meaning '(the) Beloved' or '(the) Free'. Nearly all sources portray her as the wife of the god Odin.

In Old High German and Old Norse sources, she is specifically connected with Fulla, but she is also associated with the goddesses Lofn, Hlín, Gná, and ambiguously with the Earth, otherwise personified as an apparently separate entity Jörð (Old Norse: 'Earth'). The children of Frigg and Odin include the gleaming god Baldr. Due to significant thematic overlap, scholars have proposed a connection to the goddess Freyja.
[Wikipedia](https://en.wikipedia.org/wiki/Frigg)


## Building Frigg

### Cargo

#### Prerequisites
* [Rust](https://www.rust-lang.org/tools/install/)

#### Build Executable

Navigate into the 'drasil/vidar' folder and run `cargo build`. 
If you want to build an executable for production use for example: 

`RUSTFLAGS='-C target-feature=-crt-static' cargo build --target x86_64-unknown-linux-gnu --release`

or

`RUSTFLAGS='-C target-feature=+crt-static' cargo build --target x86_64-unknown-linux-musl --release`


### Docker

#### Prerequisites
* [Rust](https://www.rust-lang.org/tools/install/)
* [Docker](https://docs.docker.com/engine/install/)

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
Frigg needs the following environment variables set to run properly:

Reward Database: 

`REWARDS_DB_URL=postgres://user:password@192.168.0.1/drasil_reward_database?password=mysecure_password`


Platform Database:

`PLATFORM_DB_URL=postgres://user:password@192.168.0.3/drasil_plattform_database?password=mysecure_password`


Public Key for JWT Certificate:

`JWT_PUB_KEY=`

...

#### Optional
For log output activate RUST_LOG by settign the trace level, default is error. 

`RUST_LOG=info`
