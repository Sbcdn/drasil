//! This module defines the configuration data for WorldMobile
//! smart contracts.

use cardano_serialization_lib::{plutus::PlutusScript, AssetName, PolicyID};
use serde::Deserialize;
use std::collections::HashMap;

/// This type defines the staking smart contract configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct StakingConfig {
    /// The WMT asset name
    pub wmt_assetname: AssetName,
    /// The WMT policy ID
    pub wmt_policy_id: PolicyID,
    /// The earth node NFT policy ID
    pub ennft_policy_id: PolicyID,
    /// Smart Contract Address of the Staking Smart Contract
    pub registration_sc_address: String,
    /// This field contains the minting and validation scripts.
    pub smart_contracts: HashMap<String, PlutusScript>,
    /// Protocol parameters path
    pub protocol_param_path: String,
}

/// This type defines the registration smart contract configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct RegistrationConfig {
    /// Registration policy
    pub policy: String,
    /// Registration contract.
    pub contract: String,
}

impl RegistrationConfig {
    /// Load registration smart contract configuration.
    pub fn load() -> Self {
        let contract = std::env::var("ENREG_CONTRACT").unwrap_or_else(|_| "".to_string());
        let policy = std::env::var("ENNFT_POLICY").unwrap_or_else(|_| "".to_string());
        Self { contract, policy }
    }
}

impl StakingConfig {
    /// Load staking smart contract configuration.
    pub fn load() -> Self {
        let wmt_assetname = AssetName::from_hex("744452415341").unwrap();
        let wmt_policy_id =
            PolicyID::from_hex("3f1d9bd2f8c3d7d8144b789433261370eaf25cdc83fe8a745ef880c1").unwrap();
        let ennft_policy_id =
            PolicyID::from_hex("d8bebcb0abd89193874c59ed3023f5b4f81b89b6676d187ad7fbdb0e").unwrap();
        let registration_sc_address =
            String::from("addr_test1wzffrh2eu39sfrmty89zma353yulrjtnphx533xz8985jpsdfa2t3");
        let mut smart_contracts = HashMap::with_capacity(2);
        let execution_proof_minting_policy = PlutusScript::from_bytes_v2(hex::decode(std::env::var("ENNFT_POLICY").unwrap_or_else(|_| "59131b591318010000332323232332232323232332232323232323232323232332232323232323232323232323233223232323232323232232323232232323232232325335323232335002222323232533553353500222222222222233355302a120013502d503025335333573466e3c048d400488d40088800811c1184ccd5cd19b8701135001223500222001047046104600c103a1335738920113746e206f7574707574206e6f74207370656e7400039150011533533573892110636865636b546e5554784f5370656e7400039150011303849854cd4d402088c888c8c8c8c94cd4ccd5cd19b873500a22222222222233302d3335530321200133503622335530391200123500122335505a0023355303c1200123500122335505d002333500123305a4800000488cc16c0080048cc1680052000001335530391200123500122335505a002333500123355303d1200123500122335505e00235503d0010012233355503704000200123355303d1200123500122335505e00235503c00100133355503203b0020015054235001222200300a017019333021500200b00d042041150011533533573892011d65787020746f6b656e206e6f74206d696e746564206f72207370656e74000411500110411533533573866e59240129436f756c64206e6f742066696e6420454e4e465420696e207265666572656e636520696e7075743a20003732a00c666ae68cdc419981028009a8039100128030068200208a8010a99a99ab9c49010b6c737420213d2073616d740004015002104013550022200115335323232323232353333573466e1cd55cea802a400046666444424666600200a0080060046eb8d5d0a8029bae35742a0086eb8d5d0a8019bae357426ae89400c8c98c813ccd5ce0218278269119111a98150031111119191299a999ab9a3371e01203a0a40a22a0042a66a66ae712410d57726f6e67207345787072546e0005115002105115335333573466e3c02006414013c5400454cd4cd5ce24810d57726f6e6720734578507243730004f15001104f15335333573466e3c02800413c1385402054cd4cd5ce24810f57726f6e672073776d74454e4e46540004e15008104e153353500f22222222222253353335530371200133503b225335002210031001504625335333573466e3c03800415014c4d41200045411c010841504148411c4cd5ce2490f57726f6e67207369676e6174757265000465335330220133500c2222222222223355302d120012350012200100b130464988854cd40044008884c12926135744a00226ae8940044d55cf280089baa00135500122002103f13357389201197374616b6520646174756d20636865636b73206661696c65640003e153353500622222222222233355302d12001223500222223500422335002200825335333573466e3c00405814c1484cd4160cd54168014018020402141400284c98c811ccd5ce248128636f756c64206e6f742066696e64207374616b696e672076616c696461746f72206f7574707574730004722135002225333500213043498854cd40104cd5ce248102766100335504a001002221304649884c11126135001220011039153353357389211176616c756550616964546f53637269707400038103913037498d401488008c08c0108c8c8c8cccd400480e08c94cd4c8ccd5cd19b8733301a001003007480040ec0e8d40108888888888880205400454cd4cd5ce24811570726f6f664f66457865637574696f6e4275726e74000391500110391533535008222232333573466e1cccc074c94cd4c0080044c10d26221533500113500222220032213047498d4020888888888888030024029200203e03d32001355047225335001150452215335333573466e24d400888d40048888ccc09800cd403488008d4034880052002040041135002223500122223500422335002233355302d12001504f300d00a505025335333573466e3c0040381281244ccd54c0b448005413cc034028d40ac488cc0080280044ccd54c0b448005413cc03402941404ccd54c090480054118c010005411c4cc09001000454cd4cd5ce248113676574457865637574696f6e50726f6f66544e000381330240040011038203820383500422001350032200230210023333573466e1cd55cea80224000466442466002006004646464646464646464646464646666ae68cdc39aab9d500c480008cccccccccccc88888888888848cccccccccccc00403403002c02802402001c01801401000c008cd40bc0c0d5d0a80619a8178181aba1500b33502f03135742a014666aa066eb940c8d5d0a804999aa819bae503235742a01066a05e0746ae85401cccd540cc0edd69aba150063232323333573466e1cd55cea801240004664424660020060046464646666ae68cdc39aab9d5002480008cc8848cc00400c008cd4115d69aba150023046357426ae8940088c98c814ccd5ce02382982889aab9e5001137540026ae854008c8c8c8cccd5cd19b8735573aa004900011991091980080180119a822bad35742a004608c6ae84d5d1280111931902999ab9c047053051135573ca00226ea8004d5d09aba2500223263204f33573808609e09a26aae7940044dd50009aba1500533502f75c6ae854010ccd540cc0dc8004d5d0a801999aa819bae200135742a00460726ae84d5d1280111931902599ab9c03f04b049135744a00226ae8940044d5d1280089aba25001135744a00226ae8940044d5d1280089aba25001135744a00226ae8940044d55cf280089baa00135742a00860526ae84d5d1280211931901e99ab9c03103d03b3333573466e1d401520022321223001003375c6ae84d55cf280491999ab9a3370ea00c900011999110911998010028020019bad35742a0126eb8d5d0a8041bad357426ae8940208c98c80f4cd5ce01881e81d81d1999ab9a3370e6aae7540312000233332222123333001005004003002375c6ae854030c8c8c8cccd5cd19b8735573aa0049000119aa81e9bae35742a0046eb8d5d09aba2500223263203f33573806607e07a26aae7940044dd50009aba1500b375c6ae854028dd71aba135744a014464c6407666ae700bc0ec0e440e84c98c80e8cd5ce249035054350003a135573ca00226ea80044d55cea80189aba25001135573ca00226ea80044d5d1280089aba25001135573ca00226ea8004c8004d540b48844894cd4004540b4884cd40b8c010008cd54c01848004010004c8004d540b08894cd4004540ac88c8c84d40188888d40208888d401088cd40088c031262533350051300c4988c8c854cd4ccd5cd19b8f00c004037036150011533533573892011557726f6e6720526567697374726174696f6e2053430003615001133355301a12001503c5010503d1533535013222235301a0062222225335333573466e1cccc07c048028009200204003f10401335738920127436f756c64206e6f742066696e6420454e4e465420696e207265666572656e636520696e7075740003f15001153353357389211c70726f626c656d2077697468207265666572656e636520696e7075740003515001133355301912001503b500f503c133355301812001503a500e350161223300200300121300d4988ccd54c0304800540b94008cd5ce248114636f756c64206e6f74206d61746368205478496e00502f133005004001222323230010053200135502f223350014800088d4008894cd4ccd5cd19b8f00200902b02a13007001130060033200135502e223350014800088d4008894cd4ccd5cd19b8f00200702a02910011300600323232323232323333573466e1cd55cea8032400046666664444442466666600200e00c00a0080060046eb8d5d0a8031bae35742a00a6eb8d5d0a8021bae35742a0066eb8d5d0a8011bae357426ae8940088c98c80c4cd5ce01281881789aba25001135744a00226ae8940044d5d1280089aab9e5001137540022466a002a04aa04c222444666aa600824002a04c66aa60142400246a0024466aa0560046aa014002666aa600824002446a00444a66a666aa6012240026a018a01e46a002446601400400a00c2006266a054008006a04e00266aa60142400246a002446466aa058006600200a640026aa05c44a66a00226aa0160064426a00444a66a6601800401022444660040140082600c006004640026aa04e4422444a66a00220044426600a004666aa600e2400200a008002640026aa04c4422444a66a00226a00644002442666a00a440046008004666aa600e2400200a00800222424446006008224244460020082466a00444666a006440040040026a002440022442466002006004640026aa042442244a66a0022a04244266a044600800466aa600c240020080022246600244a66a0042032200202c44666ae68cdc780100080b80b11a800911999a80091931901099ab9c491024c680002120012326320213357389201024c68000212326320213357389201024c68000211232230023758002640026aa03c446666aae7c004940748cd4070c010d5d080118019aba200201f232323333573466e1cd55cea8012400046644246600200600460186ae854008c014d5d09aba2500223263201f33573802603e03a26aae7940044dd50009191919191999ab9a3370e6aae75401120002333322221233330010050040030023232323333573466e1cd55cea80124000466442466002006004602a6ae854008cd4034050d5d09aba2500223263202433573803004804426aae7940044dd50009aba150043335500875ca00e6ae85400cc8c8c8cccd5cd19b875001480108c84888c008010d5d09aab9e500323333573466e1d4009200223212223001004375c6ae84d55cf280211999ab9a3370ea00690001091100191931901319ab9c01a026024023022135573aa00226ea8004d5d0a80119a804bae357426ae8940088c98c8080cd5ce00a01000f09aba25001135744a00226aae7940044dd5000899aa800bae75a224464460046eac004c8004d5406c88c8cccd55cf8011280d919a80d19aa80e18031aab9d5002300535573ca00460086ae8800c0744d5d080089119191999ab9a3370ea002900011a80398029aba135573ca00646666ae68cdc3a801240044a00e464c6403a66ae7004407406c0684d55cea80089baa0011212230020031122001232323333573466e1d400520062321222230040053007357426aae79400c8cccd5cd19b875002480108c848888c008014c024d5d09aab9e500423333573466e1d400d20022321222230010053007357426aae7940148cccd5cd19b875004480008c848888c00c014dd71aba135573ca00c464c6403666ae7003c06c06406005c0584d55cea80089baa001232323333573466e1cd55cea80124000466442466002006004600a6ae854008dd69aba135744a004464c6402e66ae7002c05c0544d55cf280089baa0012323333573466e1cd55cea800a400046eb8d5d09aab9e500223263201533573801202a02626ea80048c8c8c8c8c8cccd5cd19b8750014803084888888800c8cccd5cd19b875002480288488888880108cccd5cd19b875003480208cc8848888888cc004024020dd71aba15005375a6ae84d5d1280291999ab9a3370ea00890031199109111111198010048041bae35742a00e6eb8d5d09aba2500723333573466e1d40152004233221222222233006009008300c35742a0126eb8d5d09aba2500923333573466e1d40192002232122222223007008300d357426aae79402c8cccd5cd19b875007480008c848888888c014020c038d5d09aab9e500c23263201e33573802403c03803603403203002e02c26aae7540104d55cf280189aab9e5002135573ca00226ea80048c8c8c8c8cccd5cd19b875001480088ccc888488ccc00401401000cdd69aba15004375a6ae85400cdd69aba135744a00646666ae68cdc3a80124000464244600400660106ae84d55cf280311931900b99ab9c00b017015014135573aa00626ae8940044d55cf280089baa001232323333573466e1d400520022321223001003375c6ae84d55cf280191999ab9a3370ea004900011909118010019bae357426aae7940108c98c8050cd5ce00400a00900889aab9d50011375400224464646666ae68cdc3a800a40084244400246666ae68cdc3a8012400446424446006008600c6ae84d55cf280211999ab9a3370ea00690001091100111931900a99ab9c009015013012011135573aa00226ea80048c8cccd5cd19b87500148008801c8cccd5cd19b87500248000801c8c98c8044cd5ce00280880780709aab9d375400292103505431002335738921096f7468657277697365000021220021220012326320093357389201266d6f7265207468616e206f6e65207374616b696e672076616c696461746f72206f757470757400009232632008335738920116636865636b2045617274684e6f6465206661696c6564000082233700004002464c6400c66ae71241226d6f7265207468616e206f6e652073637269707420696e707574206f72206e6f6e65000061122002122122330010040031122123300100300249848004448c8c00400488cc00cc00800800530187d8799f581cd8bebcb0abd89193874c59ed3023f5b4f81b89b6676d187ad7fbdb0ed8799f581c3f1d9bd2f8c3d7d8144b789433261370eaf25cdc83fe8a745ef880c146744452415341ff581c9291dd59e44b048f6b21ca2df6348939f1c9730dcd48c4c2394f4906581c2dd3151fbbaf081f34345ac4e1042ee3722b441ea0371ad85bfd4a6dff0001".to_string())).unwrap()).unwrap();
        let staking_validator_smart_contract = PlutusScript::from_bytes_v2(hex::decode(std::env::var("ENNFT_POLICY").unwrap_or_else(|_| "590d08590d05010000332323232332232323232323232323322323322323232323232323232323232323232322232323232232253353333573466e1d40092002212200223333573466e1d400d2000212200123263202c33573805a05805405226a602601244444a66a660206a603200c446a004444444444444008002205e26a02c02e2a66a64646464646a00844446464a66a6602c6a012446a00444444444444400800a2a0022a66a66ae71241176f776e6572207369676e6174757265206d697373696e6700032150011032153355335533530160082135001223500122223500f223500222222222222233355302e120012235002222253353501822350062232335005233500425335333573466e3c00800415014c5400c414c814c8cd4010814c94cd4ccd5cd19b8f002001054053150031053153350032153350022133500223350022335002233500223303f00200120562335002205623303f00200122205622233500420562225335333573466e1c01800c16416054cd4ccd5cd19b870050020590581333573466e1c01000416416041604160414454cd40048414441444cd40fc018014401540e80284c98c80d0cd5ce249024c6600034103222132632036335738920120436f6e74696e75696e67206f7574707574206973206e6f7420616c6c6f7765640003615001153353357389210d736372697074206f7574707574000311500110311533532333573466e1cccc05000400c0092001032031350072235002222222222222008150051533533573892115657865637574696f6e2070726f6f66206572726f7200030150051030153353500422223223500922225335333573466e1cccc061402c0080052002036035150061533533573892011f63616e6e6f742066696e6420657865637574696f6e2070726f6f66204e46540003515006103515335333573466e20ccc0494014008005200202f030103013357389201166e6f20574d54206f6e207374616b696e67205554784f0002f102c133573892010c77726f6e6720746f6b656e730002b15335300f00121350012235001222200313263202d335738921194e6f2053637269707420496e707574732064657465637465640002d30160033333573466e1cd55cea8042400046666444424666600200a0080060046eb8d5d0a8041bae35742a00e6eb8d5d0a8031bae357426ae8940188c98c80b0cd5ce0168160151809804881509a80880909aab9d375400226ae8940044d5d1280089aab9e5001137540024446464600200a640026aa04c4466a0029000111a80111299a999ab9a3371e00401205004e2600e0022600c006640026aa04a4466a0029000111a80111299a999ab9a3371e00400e04e04c20022600c00644a66a666aa600a24002a0084a66a666ae68cdc780100081000f89a80b0008a80a80110810080f11a800911a8011111111111111999a8069281092810928109199aa980909000a80891a80091299aa99a999ab9a3371e6a004440046a0084400405e05c2666ae68cdc39a801110009a80211000817817081709a8128018a812006899091980091299a8011080188008012808190009aa80f1108911299a80089a80191000910999a802910011802001199aa980389000802802000990009aa80e9108911299a800880111099802801199aa98038900080280200091199ab9a3371e0040020340322464c6403466ae7000406924010350543500232323232323333573466e1cd55cea802a400046666644444246666600200c00a0080060046eb8d5d0a8029bae35742a0086eb8d5d0a8019bae35742a0046eb8d5d09aba2500223263201e33573803e03c03826ae8940044d5d1280089aba25001135573ca00226ea80048c8c8cccd5cd19b8735573aa0049000119910919800801801191919191919191919191919191999ab9a3370e6aae754031200023333333333332222222222221233333333333300100d00c00b00a00900800700600500400300233501301435742a01866a0260286ae85402ccd404c054d5d0a805199aa80bbae501635742a012666aa02eeb94058d5d0a80419a8098101aba150073335501702175a6ae854018c8c8c8cccd5cd19b8735573aa00490001199109198008018011919191999ab9a3370e6aae754009200023322123300100300233502b75a6ae854008c0b0d5d09aba2500223263203033573806206005c26aae7940044dd50009aba150023232323333573466e1cd55cea8012400046644246600200600466a056eb4d5d0a80118161aba135744a004464c6406066ae700c40c00b84d55cf280089baa001357426ae8940088c98c80b0cd5ce01681601509aab9e5001137540026ae854014cd404dd71aba150043335501701d200135742a006666aa02eeb88004d5d0a801180f9aba135744a004464c6405066ae700a40a00984d5d1280089aba25001135744a00226ae8940044d5d1280089aba25001135744a00226ae8940044d5d1280089aba25001135573ca00226ea8004d5d0a80118079aba135744a004464c6403466ae7006c0680604d55cf280089baa0011232230023758002640026aa030446666aae7c004940288cd4024c010d5d080118019aba2002018232323333573466e1cd55cea80124000466442466002006004601c6ae854008c014d5d09aba2500223263201833573803203002c26aae7940044dd50009191919191999ab9a3370e6aae75401120002333322221233330010050040030023232323333573466e1cd55cea80124000466442466002006004602e6ae854008cd403c058d5d09aba2500223263201d33573803c03a03626aae7940044dd50009aba150043335500875ca00e6ae85400cc8c8c8cccd5cd19b875001480108c84888c008010d5d09aab9e500323333573466e1d4009200223212223001004375c6ae84d55cf280211999ab9a3370ea00690001091100191931900f99ab9c02001f01d01c01b135573aa00226ea8004d5d0a80119a805bae357426ae8940088c98c8064cd5ce00d00c80b89aba25001135744a00226aae7940044dd5000899aa800bae75a224464460046eac004c8004d5405488c8cccd55cf80112804119a8039991091980080180118031aab9d5002300535573ca00460086ae8800c0584d5d080088910010910911980080200189119191999ab9a3370ea002900011a80398029aba135573ca00646666ae68cdc3a801240044a00e464c6402866ae700540500480444d55cea80089baa0011212230020031122001232323333573466e1d400520062321222230040053007357426aae79400c8cccd5cd19b875002480108c848888c008014c024d5d09aab9e500423333573466e1d400d20022321222230010053007357426aae7940148cccd5cd19b875004480008c848888c00c014dd71aba135573ca00c464c6402466ae7004c04804003c0380344d55cea80089baa001232323333573466e1cd55cea80124000466442466002006004600a6ae854008dd69aba135744a004464c6401c66ae7003c0380304d55cf280089baa0012323333573466e1cd55cea800a400046eb8d5d09aab9e500223263200c33573801a01801426ea80048c8c8c8c8c8cccd5cd19b8750014803084888888800c8cccd5cd19b875002480288488888880108cccd5cd19b875003480208cc8848888888cc004024020dd71aba15005375a6ae84d5d1280291999ab9a3370ea00890031199109111111198010048041bae35742a00e6eb8d5d09aba2500723333573466e1d40152004233221222222233006009008300c35742a0126eb8d5d09aba2500923333573466e1d40192002232122222223007008300d357426aae79402c8cccd5cd19b875007480008c848888888c014020c038d5d09aab9e500c23263201533573802c02a02602402202001e01c01a26aae7540104d55cf280189aab9e5002135573ca00226ea80048c8c8c8c8cccd5cd19b875001480088ccc888488ccc00401401000cdd69aba15004375a6ae85400cdd69aba135744a00646666ae68cdc3a80124000464244600400660106ae84d55cf280311931900719ab9c00f00e00c00b135573aa00626ae8940044d55cf280089baa001232323333573466e1d400520022321223001003375c6ae84d55cf280191999ab9a3370ea004900011909118010019bae357426aae7940108c98c802ccd5ce00600580480409aab9d50011375400224464646666ae68cdc3a800a40084244400246666ae68cdc3a8012400446424446006008600c6ae84d55cf280211999ab9a3370ea00690001091100111931900619ab9c00d00c00a009008135573aa00226ea80048c8cccd5cd19b8750014800880148cccd5cd19b8750024800080148c98c8020cd5ce00480400300289aab9d37540022440042440029309000a490350543100112323001001223300330020020014c183d8799f581cd8bebcb0abd89193874c59ed3023f5b4f81b89b6676d187ad7fbdb0e581c9291dd59e44b048f6b21ca2df6348939f1c9730dcd48c4c2394f4906581c3f1d9bd2f8c3d7d8144b789433261370eaf25cdc83fe8a745ef880c146744452415341581cab194d7747d40b17c96580e417a94ba699f98a91e44c4fc0d6f4d2bdff0001".to_string())).unwrap()).unwrap();
        smart_contracts.insert(String::from("validator"), execution_proof_minting_policy);
        smart_contracts.insert(String::from("minting"), staking_validator_smart_contract);
        let protocol_param_path = std::env::var("PPPATH")
            .unwrap_or_else(|_| "protocol_parameters_preview.json".to_owned());
        Self {
            wmt_assetname,
            wmt_policy_id,
            ennft_policy_id,
            registration_sc_address,
            smart_contracts,
            protocol_param_path,
        }
    }
}
