# Odin

## Summary
Odin is heart of the drasil system, it builds an finalize transactions in general and interacts and composes the different parts. 
It implements the blockchain applications and is the central runtime of the Drasil application framework. 
Odin itself is very lightwight but implements via 'Commands' many different blockchain interactions. The communications protocol is 
created with the Redis ..... 

There is no direct documentation for Odin as it serves basically all existing functions. Odin is a internal service and its not possible to directly 
communicate with Odin. The API to communicate with Odin is 'Heimdallr'. This allows encapsulations between the central blockchain services and the available endpoints as well as independent scalability between the two services. 
By using a loadbalancer between several Heimdallr and Odin instances it is possible to scale the application depending on load. Each Odin instance starts incomming 
application commands as asynchronous independent processes. 

* [Documentation](https://docs.drasil.io/)

* [License]( https://www.drasil.io/licenses/LICENSE-1.0)

## Mythology
Odin (/ˈoʊdɪn/; from Old Norse: Óðinn, IPA: [ˈoːðenː]) is a widely revered god in Germanic paganism. Norse mythology, the source of most surviving information about him, associates him with wisdom, healing, death, royalty, the gallows, knowledge, war, battle, victory, sorcery, poetry, frenzy, and the runic alphabet, and depicts him as the husband of the goddess Frigg. In wider Germanic mythology and paganism, the god was also known in Old English as Wōden, in Old Saxon as Uuôden, in Old Dutch as Wuodan, in Old Frisian as Wêda, and in Old High German as Wuotan, all ultimately stemming from the Proto-Germanic theonym *Wōđanaz, meaning 'lord of frenzy', or 'leader of the possessed'.

Odin appears as a prominent god throughout the recorded history of Northern Europe, from the Roman occupation of regions of Germania (from c.  2 BCE) through movement of peoples during the Migration Period (4th to 6th centuries CE) and the Viking Age (8th to 11th centuries CE). In the modern period the rural folklore of Germanic Europe continued to acknowledge Odin. References to him appear in place names throughout regions historically inhabited by the ancient Germanic peoples, and the day of the week Wednesday bears his name in many Germanic languages, including in English.
[Wikipedia](https://en.wikipedia.org/wiki/Odin)


## Building Odin

### Cargo

#### Prerequisites
* [Rust](https://www.rust-lang.org/tools/install/)

#### Build Executable

Navigate into the 'drasil/odin' folder and run `cargo build`. 
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

REDIS_DB
REDIS_DB_URL_UTXOMIND
REDIS_DB_URL_REPLIC
REDIS_CLUSTER
TX_SUBMIT_ENDPOINT1
TX_SUBMIT_ENDPOINT2
TX_SUBMIT_ENDPOINT3
USED_UTXO_DATASTORE_1
USED_UTXO_DATASTORE_2
USED_UTXO_DATASTORE_3


Delete: 
SYSTEM_FEE_MINT
SYSTEM_FEE_WALLET
PVKSECRET
SYSTEM_FEE_RWD
SYSTEM_FEE_WALLET
APPLY_SYSTEM_FEE


#### Optional
For log output activate RUST_LOG by settign the trace level, default is error. 

`RUST_LOG=info`

