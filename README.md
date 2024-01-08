# Drasil

Drasil is an opinionated implementation of a software suite to run applications on Cardano, it is a collection of microservices which, when used together, form an effective and scalable framework for running applications on the Cardano blockchain. Its architecture, services and libraries are written in Rust. 

# The codebase is still under development and not considered production ready, use on own risk. 

Drasil system and concept was created by Torben and Zak, but it also utilises several tools developed by the Cardano community, to whom we are grateful and hope to continue to repay with our own small contributions.  Because of the way in which the various tools interact, more is required to actually run a fully integrated and working "Drasil System" than just the applications and libraries found in this repository! (Redis, RabbitMQ, Cardano-Node, Dbsync, Oura, Postgres)

Drasil is made for Orechstration, can run in native or managed Kubernetes, on native Linux or locally in your docker environment. 
You can read the original blackpaper [here](https://bit.ly/3vg9GvI)

* [API Documentation Local Cluster] (https://documenter.getpostman.com/view/23201834/2s9YXpUHwG)
* [License] (https://github.com/sbcdn/drasil/License.md)

The web interface for the administration via frigg's REST Api is in development. 

If you have questions or need support, please [contact us](mailto:info@drasil.io) us. 

## Supported Applications 
- NFT Collection Minter
- Reward System
- NFT Marketplace

## Additional Features
- OneShotMinter API
- Wallet Backend to build Standard Transactions (Asset Transfers)
- Ada staking

## Custom Transaction Building
You can use the drasil system to build your own transactions, for that the custom transaction building procedure must be implemented via modules. 

## Building and Testing Drasil
The most convinient way to test and run drasil is via the local cluster. 
Please find a detailed description in the [local setup](https://github.com/sbcdn/drasil/local/)
It using a local Kubernetes setup utilizing k3d and docker. The deployment configuration can be found in 'local', be aware that the local setup is not suitable for production use, additional security measures must be taken. 

The connection string to dbsync must be amended in  'local/configmaps/drasil_configmap.yaml' before starting a local cluster or via envrionment variable 'DBSYNC_DB_URL' on the container in kubernetes. 
The local setup will NOT spin up a cardano node and dbsync, this has to be done seperatly or connect to an existing one. 

### Oura
Oura must be configured to fill a redis stream for transactions. You can find a detailed description how to setup redis streams with oura [here](https://github.com/txpipe/oura)

## Quick Overview about the libraries and services
- libraries
   - drasil-dvltath : Library to build a sidecar container to use Hashicorp Vault as Private Key Storage for "odin" and "frigg".
   - drasil-gugnir : Library to manage the rewards database
   - drasil-hugin : General protocol library
   - drasil-mimir : Dbsync interface, will be replaced by the cardano data provider soon
   - drasil-murin : Cardano Transaction building library working together with hugin
   - drasil-sleipnir : Administrator functions library, here is all what "frigg" needs to create new wallets or an NFT collection, ...

- services are the microservices runnning in the cluster
   - frigg: Admin Backend-Server for the admin web interface (web interface is in development...)
   - heimdallr: Transaction Building Gateway to Odin, web facing endpoint
   - vidar : REST API to retrieve Reward and Minting information
   - odin : runs the core library as a service, transaction building, authentication, private key handling etc. the only service allowed to interact with odin is heimdallr.  
   - loki : Websocket for NFT Minting and other asynchronous user interactions via jobs and queues
   - geri : Cardano Chain Follower and Clean Up System for redis cache and pending UTxO memory
   - jobs : general job processor service for example to perform long lasting database operations
   - work_loki : minting system worker (is its own service so we can run it isolated from the other jobs)

   Additionally it needs a Redis database, two Postgres Databases, RabbitMQ, a Cardano Node and DBsync and an instance of Oura.

### Name and Mythology
The word "Drasil" derives from "Yggdrasil" which described the "world tree" in Norse mythology, although there are several different transalations and interpretations, some of which you can find below.

Yggdrasil (from Old Norse Yggdrasill [ˈyɡːˌdrɑselː]), in Norse cosmology, is an immense and central sacred tree. Around it exists all else, including the Nine Worlds.

Yggdrasil is attested in the Poetic Edda compiled in the 13th century from earlier traditional sources, and in the Prose Edda written in the 13th century by Snorri Sturluson. In both sources, Yggdrasil is an immense ash tree that is center to the cosmos and considered very holy. The gods go to Yggdrasil daily to assemble at their traditional governing assemblies, called things. The branches of Yggdrasil extend far into the heavens, and the tree is supported by three roots that extend far away into other locations; one to the well Urðarbrunnr in the heavens, one to the spring Hvergelmir, and another to the well Mímisbrunnr. Creatures live within Yggdrasil, including the dragon Níðhöggr, an unnamed eagle, and the stags Dáinn, Dvalinn, Duneyrr and Duraþrór.

Scholars generally consider Hoddmímis holt, Mímameiðr, and Læraðr to be other names for the tree. The tree is an example of sacred trees and groves in Germanic paganism and mythology,...

The generally accepted meaning of Old Norse Yggdrasill is "Odin's horse", meaning "gallows". This interpretation comes about because drasill means "horse" and Ygg(r) is one of Odin's many names. The Poetic Edda poem Hávamál describes how Odin sacrificed himself by hanging from a tree, making this tree Odin's gallows. This tree may have been Yggdrasil. Gallows can be called "the horse of the hanged" and therefore Odin's gallows may have developed into the expression "Odin's horse", which then became the name of the tree.

Nevertheless, scholarly opinions regarding the precise meaning of the name Yggdrasill vary, particularly on the issue of whether Yggdrasill is the name of the tree itself or if only the full term askr Yggdrasil (where Old Norse askr means "ash tree") refers specifically to the tree. According to this interpretation, askr Yggdrasils would mean the world tree upon which "the horse [Odin's horse] of the highest god [Odin] is bound". Both of these etymologies rely on a presumed but unattested *Yggsdrasill.

A third interpretation, presented by F. Detter, is that the name Yggdrasill refers to the word yggr ("terror"), yet not in reference to the Odinic name, and so Yggdrasill would then mean "tree of terror, gallows". F. R. Schröder has proposed a fourth etymology according to which yggdrasill means "yew pillar", deriving yggia from *igwja (meaning "yew-tree"), and drasill from *dher- (meaning "support").
[Wikipedia](https://en.wikipedia.org/wiki/Yggdrasil)

