# Freki

## Summary
Freki is a reward calculator and has to run in a cron job once per epoch.

* [Documentation](https://docs.drasil.io/reward-and-distribution/drasil-rewards-api/reward-information)


## Mythology
In Norse mythology, Geri and Freki (Old Norse, both meaning "the ravenous" or "greedy one") are two wolves which are said to accompany the god Odin. They are attested in the Poetic Edda, a collection of epic poetry compiled in the 13th century from earlier traditional sources, in the Prose Edda, written in the 13th century by Snorri Sturluson, and in the poetry of skalds. The pair has been compared to similar figures found in Greek, Roman and Vedic mythology, and may also be connected to beliefs surrounding the Germanic "wolf-warrior bands", the Úlfhéðnar.
[Wikipedia](https://en.wikipedia.org/wiki/Geri_and_Freki)


## Building Freki

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
Freki needs the following environment variables set to run properly:

Reward Database: 

`REWARDS_DB_URL=postgres://user:password@192.168.0.1/drasil_reward_database?password=mysecure_password`


DBSync Database:

`DBSYNC_DB_URL=postgres://user:password@192.168.0.2/cexplorer?password=mysecure_password`


Platform Database:

`PLATFORM_DB_URL=postgres://user:password@192.168.0.3/drasil_plattform_database?password=mysecure_password`

#### Optional
For log output activate RUST_LOG by settign the trace level, default is error. 

`RUST_LOG=info`

