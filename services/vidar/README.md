# Vidar

## Summary
'Vidar' is the gateway to retrieve data from the reward and distribution system.  It needs API key authentication from a registered user who has the permission to use the reward and distribution system and its information. 

* [Documentation](https://docs.drasil.io/reward-and-distribution/drasil-rewards-api/reward-information)

* [License]( https://www.drasil.io/licenses/LICENSE-1.0)

## Mythology
In Norse mythology, Vidar is one of the Aesir and Odin's son. He belongs to the circle of gods whose function and meaning are somewhat problematic to circle. Even the god's name has given rise to completely different interpretations. According to one interpretation, the name means "the wide ruler", according to another "he of the forest", "the warrior of the forest" or the like. Vidar is also called "the silent ace" or "the silent god".

Vidar certainly appears in Grimnesmål as the ruler of his kingdom of Vide , the "forest land", but apart from this reference, the image of a powerful, vengeful god predominates. In Skáldskaparmál it is said that Vidar is called "Avenger of the Gods". Vidar's primary mythological function is precisely that of an avenger: at Ragnarök he avenges his father Odin. In that respect, he is functionally close to the god Vale whose mythological function was precisely that of Balder 's avenger. Odin fathered both Vidar and Vale through extramarital relations. Vidar's mother is said to be the giantess Grid .
[Wikipedia](https://sv.wikipedia.org/wiki/Vidar)


## Building Vidar

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
Vidar needs the following environment variables set to run properly:

Reward Database: 

`REWARDS_DB_URL=postgres://user:password@192.168.0.1/drasil_reward_database?password=mysecure_password`


DBSync Database:

`DBSYNC_DB_URL=postgres://user:password@192.168.0.2/cexplorer?password=mysecure_password`


Platform Database:

`PLATFORM_DB_URL=postgres://user:password@192.168.0.3/drasil_plattform_database?password=mysecure_password`


Public Key for JWT Certificate:

`JWT_PUB_KEY=`

#### Optional
For log output activate RUST_LOG by settign the trace level, default is error. 

`RUST_LOG=info`
