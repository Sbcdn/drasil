# Geri

## Summary
Geri is a worker application which checks blockchain events, updates utxo states, performs clean up task.
This service works with redis streams and Oura in combination. 

* [Documentation](https://docs.drasil.io/reward-and-distribution/drasil-rewards-api/reward-information)

* [License]( https://www.drasil.io/licenses/LICENSE-1.0)

## Mythology
In Norse mythology, Geri and Freki (Old Norse, both meaning "the ravenous" or "greedy one") are two wolves which are said to accompany the god Odin. They are attested in the Poetic Edda, a collection of epic poetry compiled in the 13th century from earlier traditional sources, in the Prose Edda, written in the 13th century by Snorri Sturluson, and in the poetry of skalds. The pair has been compared to similar figures found in Greek, Roman and Vedic mythology, and may also be connected to beliefs surrounding the Germanic "wolf-warrior bands", the Úlfhéðnar.
[Wikipedia](https://en.wikipedia.org/wiki/Geri_and_Freki)

## Oura
- [Oura](https://github.com/txpipe/oura)

## Building Geri

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
Geri needs the following environment variables set to run properly:

...

#### Optional
For log output activate RUST_LOG by settign the trace level, default is error. 

`RUST_LOG=info`
