# Heimdallr

## Summary
'Heimdallr' is the gateway to interact with the blockchain system in general and it guards a bridge to 'Odin'. 
It requires API key authentication from a registered user who has permission to use the system.
The transaction builder as well as infrastructure APIs are made accessible by 'Heimdallr'. 

* [Documentation](https://docs.drasil.io/)


## Mythology
In Norse mythology, Heimdall (from Old Norse Heimdallr, [ˈhɛimˌdɑlːz̠]) is a god who keeps watch for invaders and the onset of Ragnarök from his dwelling Himinbjörg, where the burning rainbow bridge Bifröst meets the sky. He is attested as possessing foreknowledge and keen senses, particularly eyesight and hearing. The god and his possessions are described in enigmatic manners. For example, Heimdall is gold-toothed, "the head is called his sword," and he is "the whitest of the gods."

Heimdall possesses the resounding horn Gjallarhorn and the golden-maned horse Gulltoppr, along with a store of mead at his dwelling. He is the son of Odin and the Nine Mothers, and he is said to be the originator of social classes among humanity. Other notable stories include the recovery of Freyja's treasured possession Brísingamen while doing battle in the shape of a seal with Loki. The antagonistic relationship between Heimdall and Loki is notable, as they are foretold to kill one another during the events of Ragnarök.
[Wikipedia](https://en.wikipedia.org/wiki/Heimdall)


## Building Heimdallr

### Cargo

#### Prerequisites
* [Rust](https://www.rust-lang.org/tools/install/)

#### Build Executable

Navigate into the 'drasil/vidar' folder and run `cargo build`. 
Use the following if you want to build an executable for production use (for example): 

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
'Heimdallr' needs the following environment variables set to run properly:

Reward Database: 

`REWARDS_DB_URL=postgres://user:password@192.168.0.1/drasil_reward_database?password=mysecure_password`


DBSync Database:

`DBSYNC_DB_URL=postgres://user:password@192.168.0.2/cexplorer?password=mysecure_password`


Platform Database:

`PLATFORM_DB_URL=postgres://user:password@192.168.0.3/drasil_plattform_database?password=mysecure_password`


Public Key for JWT Certificate:

`JWT_PUB_KEY=`


....
#### Optional
For log output activate RUST_LOG by setting the trace level (default is error). 

`RUST_LOG=info`