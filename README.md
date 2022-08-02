# Drasil

The Drasil Application Framework is a collection of microservices which, when used together, form an effective, scalabe framework for running dApps on the Cardano blockchain. Its architecture, services and libraries are written in Rust.

Drasil system and concept was created by Torben and Zak, but it also utilises several tools developed by the Cardano community, to whom we are grateful and hope to continue to repay with our own small contributions.  Because of the complicated way in which the various tools interact, a lot more is required to actually run a fully integrated and working "Drasil System" than just the applications and libraries found in this repository, of course!  We exist to support and advise on how to utilize this framework for particular applications and consult on what it is required to run it effectively.

Drasil is made for Orechstration, can run in native or managed Kubernetes, on native Linux or locally in your docker environment. We constantly develop Drasil and add features, smart contracts and applications. We also maintain a testnet and mainnet version, and it is possible to subscribe to single services or applications directly without running your own Drasil System.  You can read the original blackpaper [here](https://bit.ly/3vg9GvI)

Visit us at [drasil.io](https://www.drasil.io)

We also work with [Maestro](https://www.gomaestro.org) for infrastructure and managed Drasil systems. 

* [Documentation](https://docs.drasil.io/)

* [License]( https://www.drasil.io/licenses/LICENSE-1.0)

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
If you want to build executables for production use for example: 

`RUSTFLAGS='-C target-feature=-crt-static' cargo build --target x86_64-unknown-linux-gnu --release`

or

`RUSTFLAGS='-C target-feature=+crt-static' cargo build --target x86_64-unknown-linux-musl --release`


### Docker
Follow the Readme's in the single applications folders


### Environment 
Drasils individual services need many settings which are passed via environment variables, the individual needed settings are described in the Readme file for each service. 

## Architecture

...Follows soon...
