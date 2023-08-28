# Drasil

The Drasil Application Framework is a collection of microservices which, when used together, form an effective, scalable, comprehensive and powerful framework for running decentralised applications (or "dApps") on the Cardano blockchain. Its architecture, services and libraries are written in Rust.

Drasil system and concept was created by Torben and Zak, but it also utilises several tools developed by the Cardano community, to whom we are grateful and hope to continue to repay with our own small contributions.  Because of the complicated way in which the various tools interact, a lot more is required to actually run a fully integrated and working "Drasil System" than just the applications and libraries found in this repository, of course!  We exist to support and advise on how to utilize this framework for particular applications and consult on what it is required to run it effectively.

Drasil is made for Orechstration, can run in native or managed Kubernetes, on native Linux or locally in your docker environment. We are constantly developing Drasil with new features, smart contracts and applications. We also maintain a testnet and mainnet version, and it is possible to subscribe to hosted single services or applications directly without the need to run your own Drasil System.  You can read the original blackpaper [here](https://bit.ly/3vg9GvI)

Visit us at [drasil.io](https://www.drasil.io)

* [Documentation](https://docs.drasil.io/)


## Mythology
The word "Drasil" derives from "Yggdrasil" which described the "world tree" in Norse mythology, although there are several different transalations and interpretations, some of which you can find below. We chose Drasil as we imagine this application framework as providing the branches of a large tree, the "World tree" bearing Cardanos applications.

Yggdrasil (from Old Norse Yggdrasill [ˈyɡːˌdrɑselː]), in Norse cosmology, is an immense and central sacred tree. Around it exists all else, including the Nine Worlds.

Yggdrasil is attested in the Poetic Edda compiled in the 13th century from earlier traditional sources, and in the Prose Edda written in the 13th century by Snorri Sturluson. In both sources, Yggdrasil is an immense ash tree that is center to the cosmos and considered very holy. The gods go to Yggdrasil daily to assemble at their traditional governing assemblies, called things. The branches of Yggdrasil extend far into the heavens, and the tree is supported by three roots that extend far away into other locations; one to the well Urðarbrunnr in the heavens, one to the spring Hvergelmir, and another to the well Mímisbrunnr. Creatures live within Yggdrasil, including the dragon Níðhöggr, an unnamed eagle, and the stags Dáinn, Dvalinn, Duneyrr and Duraþrór.

Scholars generally consider Hoddmímis holt, Mímameiðr, and Læraðr to be other names for the tree. The tree is an example of sacred trees and groves in Germanic paganism and mythology,...

The generally accepted meaning of Old Norse Yggdrasill is "Odin's horse", meaning "gallows". This interpretation comes about because drasill means "horse" and Ygg(r) is one of Odin's many names. The Poetic Edda poem Hávamál describes how Odin sacrificed himself by hanging from a tree, making this tree Odin's gallows. This tree may have been Yggdrasil. Gallows can be called "the horse of the hanged" and therefore Odin's gallows may have developed into the expression "Odin's horse", which then became the name of the tree.

Nevertheless, scholarly opinions regarding the precise meaning of the name Yggdrasill vary, particularly on the issue of whether Yggdrasill is the name of the tree itself or if only the full term askr Yggdrasil (where Old Norse askr means "ash tree") refers specifically to the tree. According to this interpretation, askr Yggdrasils would mean the world tree upon which "the horse [Odin's horse] of the highest god [Odin] is bound". Both of these etymologies rely on a presumed but unattested *Yggsdrasill.

A third interpretation, presented by F. Detter, is that the name Yggdrasill refers to the word yggr ("terror"), yet not in reference to the Odinic name, and so Yggdrasill would then mean "tree of terror, gallows". F. R. Schröder has proposed a fourth etymology according to which yggdrasill means "yew pillar", deriving yggia from *igwja (meaning "yew-tree"), and drasill from *dher- (meaning "support").
[Wikipedia](https://en.wikipedia.org/wiki/Yggdrasil)


##

## Building Drasil

### Cargo

#### Prerequisites
* [Rust](https://www.rust-lang.org/tools/install/)

#### Build Executable

Navigate into the 'drasil' folder and run `cargo build`. This will build all services at once.
If you want to build executables for production, you can use (for example): 

`RUSTFLAGS='-C target-feature=-crt-static' cargo build --target x86_64-unknown-linux-gnu --release`

or

`RUSTFLAGS='-C target-feature=+crt-static' cargo build --target x86_64-unknown-linux-musl --release`


### Docker
Follow the Readme files in the single applications' folders


### Environment 
Drasils individual services require the setting of many parameters which are passed via environment variables.  These are described in the Readme file corresponding to the specific service. Here is a list of (probably) all the environment variables used in this repo, with placeholder values:

set_var("ADM_USER", "trsfasfue");
set_var("POW", "trsfasfue");
set_var("ODIN_URL", "trsfasfue");
set_var("PLATFORM_DB_URL", "trsfasfue");
set_var("MOUNT", "trsfasfue");
set_var("VPATH", "trsfasfue");
set_var("VSOCKET_PATH", "trsfasfue");
set_var("OROLE_ID", "trsfasfue");
set_var("OSECRET_ID", "trsfasfue");
set_var("RUST_LOG", "trsfasfue");
set_var("VAULT_TOKEN", "trsfasfue");
set_var("REWARDS_DB_URL", "trsfasfue");
set_var("DBSYNC_DB_URL", "trsfasfue");
set_var("CARDANO_CLI_PATH", "trsfasfue");
set_var("CARDANO_PROTOCOL_PARAMETER_PATH", "trsfasfue");
set_var("TX_SUBMIT_ENDPOINT1", "trsfasfue");
set_var("TX_SUBMIT_ENDPOINT2", "trsfasfue");
set_var("TX_SUBMIT_ENDPOINT3", "trsfasfue");
set_var("REDIS_DB", "redis://127.0.0.1:6379/0");
set_var("REDIS_DB_URL_UTXOMIND", "trsfasfue");
set_var("REDIS_DB_URL_REPLICA", "trsfasfue");
set_var("REDIS_CLUSTER", "false");
set_var("TXGSET", "tfsafasrue");
set_var("USED_UTXO_DATASTORE_1", "trsfasfue");
set_var("USED_UTXO_DATASTORE_2", "trfsafue");
set_var("USED_UTXO_DATASTORE_3", "trfsafsaue");
set_var("PENDING_TX_DATASTORE_1", "sfsafa");
set_var("PENDING_TX_DATASTORE_2", "fasfsaf");
set_var("PENDING_TX_DATASTORE_3", "fsafasf");
set_var("JWT_KEY", "trsfasfue");
set_var("DRASIL_REWARD_DB", "trsfasfue");
set_var("JWT_PUB_KEY", "trsfasfue");
set_var("RUST_LOG", "trsfasfue");
set_var("POD_HOST", "trsfasfue");
set_var("POD_PORT", "trsfasfue");
set_var("VERIFICATION_LINK", "trsfasfue");
set_var("SMTP_USER", "trsfasfue");
set_var("SMTP_PW", "trsfasfue");
set_var("FROM_EMAIL", "trsfasfue");
set_var("EMAIL_API_KEY", "trsfasfue");
set_var("AMQP_ADDR", "trsfasfue");
set_var("QUEUE_NAME", "trsfasfue");
set_var("CONSUMER_NAME", "trsfasfue");
set_var("JWT_PUB_KEY", "trsfasfue");
set_var("ODIN_URL", "trsfasfue");
set_var("AMWP_ADDR", "trsfasfue");
set_var("STREAM_TRIMMER", "trsfasfue");
set_var("STREAMS", "trsfasfue");
set_var("TIMEOUT", "trsfasfue");

## Architecture

...coming soon...

## Quick Guide for James

Folders:
- jobs : does include all binaries which should run as cron-jobs (is for the Reward system only)
- libs : includes all libraries, this is the core
   - dvlth : is a sidecar binary for odin and frigg, dvlth communicates with HashiCorp Vault and exchanges secrets (expire after 3s) via filesystem (temp volume mapping)
- services : are the main binaries 
   - frigg: Admin Backend-Server for th eno-code plattform
   - heimdallr: Transaction Building Gateway to Odin, web facing endpoint
   - loki : Websocket Bridge For NFT Minting and other asynchronous user interactions via jobs and queues
   - odin : runs the core library as a service, transaction building, authentication, private key handling etc. the only service allowed to interact with oding it heimdallr, odin is isolated with access to vault / system and reward database, heimdallr has no access to those databases
   - vidar : REST API to retrieve Reward and Minting information
   - wsauth : test program oyu can ignore for the moment
- worker : binaries performing work on redis, the databases or just processing jobs from the job queue
   - geri : Cardano Chain Follower and Clean Up System for redis cache and pending utxo memory
   - jobs : job processor for general drasil jobs
   - work_loki : minting system worker (is isolated from the rest, has some special needs)

   Additionally we need a Redis Database, two Postgres Databases and a DBsync (third postgres database)
   There is also the Cardano-Data-Provider which abstarcts some stuff from this libraries and take it into its own repository.
   The CDP hasits own binary (the wallet backend).
   Then there is the csl-common library which unifies some functions.

For a "simple" start go with odin, heimdallr, cdp, redis and the postgres databases that is the minimal setup.