/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Business Source License v1.0                                  #
# Open Source Release: 36 month after release date                              #
# Open Source License: GNU Public License v2.0                                  #
# Licensor: Drasil Association (info@drasil.io)                                 #
#################################################################################
*/
use super::models::*;
use crate::SleipnirError;
use chrono::{DateTime, NaiveDateTime, Utc};
use gungnir::minting::models::*;

pub async fn create_mintproject(data: &CreateMintProj) -> Result<MintProject, SleipnirError> {
    let time_constraint = if let Some(date) = &data.time_constraint {
        Some(DateTime::<Utc>::from_utc(
            NaiveDateTime::parse_from_str(date, "%Y-%m-%d %H:%M:%S")?,
            Utc,
        ))
    } else {
        None
    };

    let policy_script_id = super::create_policy_script(
        murin::get_network_kind(data.network).await?,
        data.user_id.unwrap(),
        None,
        time_constraint,
    )
    .await?;

    let mint_start_date = match data.mint_start_date {
        Some(date) => date,
        None => Utc::now(),
    };

    let mint_contract =
        hugin::TBContracts::get_contract_uid_cid(data.user_id.unwrap(), policy_script_id)?;

    let tablename = make_table_name(
        data.user_id.unwrap(),
        policy_script_id,
        &data.project_name,
        &data.collection_name,
        &mint_contract.policy_id.unwrap(),
    );
    Nft::create_nft_table(&tablename).await?;

    log::debug!("table created: {}...", tablename);
    log::debug!("try to create mint project...");
    let m = MintProject::create_mintproject(
        &data.project_name,
        &data.user_id.unwrap(),
        &policy_script_id,
        None,
        &mint_start_date,
        data.mint_end_date.as_ref(),
        &data.storage_type,
        data.storage_url.as_ref(),
        data.storage_access_token.as_ref(),
        &data.collection_name,
        &data.author,
        &data.meta_description,
        data.meta_common_nft_name.as_ref(),
        data.max_mint_p_addr.as_ref(),
        &tablename,
        &false,
    )?;
    log::debug!("...finish create mintproject");
    Ok(m)
}

pub fn make_table_name(
    user_id: i64,
    policy_script_id: i64,
    project_name: &String,
    collection_name: &String,
    policy_id: &String,
) -> String {
    let name = user_id.to_string()
        + &policy_script_id.to_string()
        + project_name
        + collection_name
        + policy_id;
    "zznft_".to_string() + &hex::encode(murin::blake2b160(name.as_bytes()))
}

pub async fn import_nfts_from_csv_metadata(
    csv: &[u8],
    user_id: i64,
    mint_pid: i64,
) -> Result<usize, SleipnirError> {
    let mut rdr = csv::Reader::from_reader(csv);
    let mut trdr = csv::Reader::from_reader(csv);
    let n = trdr.records().count();
    log::debug!("Count: {:?}", n);
    //log::debug!("Reader has headers: {:?}", rdr.has_headers());
    let mut counter = 0;
    for result in rdr.records() {
        log::debug!("A Record was found: {:?}", result);
        if let Ok(record) = result {
            for n in record.iter() {
                log::debug!("Try to parse from json...: {}", n);
                let assets = murin::minter::AssetMetadata::from_json(n)?;
                log::debug!("Try to import from asset metadata...");
                let nfts = import_from_asset_metadata(user_id, mint_pid, assets).await?;
                log::debug!("\n Total Imported Nfts: {:?}", nfts);
                counter += nfts.len();
            }
        } else {
            log::debug!("No more records; processed: {}", counter);
        }
    }
    Ok(counter)
}

pub fn find_numbers(str: &str) -> Result<String, SleipnirError> {
    let alphabet = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
    let mut out = String::new();
    for i in str[..].chars() {
        if alphabet.contains(&i) {
            out.push(i);
        }
    }
    Ok(out)
}

pub async fn import_from_asset_metadata(
    user_id: i64,
    mpid: i64,
    meta: Vec<murin::minter::AssetMetadata>,
) -> Result<Vec<gungnir::minting::models::Nft>, SleipnirError> {
    //let gconn = &mut gungnir::establish_connection()?;
    let mint_project = MintProject::get_mintproject_by_id(mpid)?;
    println!("Try to find contract...");
    let mint_contract =
        hugin::TBContracts::get_contract_uid_cid(user_id, mint_project.mint_contract_id)?;
    println!("Found contract");
    let mut nfts = Vec::<Nft>::new();
    for m in meta {
        let nft_id = m.tokenname.clone();
        //find_numbers(m.name.as_ref().unwrap())?;
        //if nft_id.is_empty() {
        //    nft_id = m.tokenname.clone();
        //}
        let asset = m.clone();
        let nft = Nft::create_nft(
            &mint_project.nft_table_name,
            &mpid,
            &hex::decode(&m.tokenname)?, //asset_name_b
            &m.name.unwrap(),            //asset_name
            &murin::make_fingerprint(mint_contract.policy_id.as_ref().unwrap(), &m.tokenname)?,
            &nft_id,
            Some(&("file_".to_string() + m.image_url.as_ref().unwrap())),
            m.image_url.as_ref(),
            Some(&serde_json::json!(asset).to_string()),
            None,
        )
        .await;
        if let Err(e) = &nft {
            log::error!("Error on NFT creation in reward database: {}", e);
        }
        nfts.push(nft?);
    }
    Ok(nfts)
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn test_csv_import() {
        std::env::set_var(
            "REWARDS_DB_URL",
            "", // set db connection string
        );
        std::env::set_var(
            "PLATFORM_DB_URL",
            "", // set db connection string
        );
        let csv = "5c6d657461646174610a227b0a2020202022226d7961737365746e616d655f757466382222203a207b0a202020202020202022226e616d6522223a22226d7961737365746e616d655f7574663822222c0a20202020202020202222696d61676522223a222262616679626569686379727561657a613775796a64367567696362637271756d656a6636756633353365356574646b686f7471666677746775766122222c0a202020202020202022226d656469615479706522223a2222696d6167652f706e6722222c0a202020202020202022226465736372697074696f6e22223a22224d7920446573637269747074696f6e20666f722074686973204e465422222c0a2020202020202020222266696c657322223a5b0a2020202020202020202020207b0a2020202020202020202020202020202022226e616d6522223a22226d7966696c656e616d653122222c0a2020202020202020202020202020202022226d656469615479706522223a222266696c652f7a697022222c0a20202020202020202020202020202020222273726322223a205b22226d79536f75726365506174682f66696c652e7a697022225d0a2020202020202020202020207d2c7b0a2020202020202020202020202020202022226e616d6522223a22226d7966696c656e616d653222222c0a2020202020202020202020202020202022226d656469615479706522223a2222766964656f2f6d6f7622222c0a20202020202020202020202020202020222273726322223a205b22226d79536f75726365506174682f7669642e6d6f7622225d0a2020202020202020202020207d0a20202020202020205d2c0a202020202020202022226f7468657222223a222270726f7065727469657322222c0a2020202020202020222274726169747322223a5b222274726169743122222c222274726169743222222c222274726169743322222c222274726169743422225d0a202020207d2c0a0a2020202022223664373936313733373336353734366536313664363535663632363936653631373237392222203a207b0a202020202020202022226e616d6522223a22226d7961737365746e616d655f62696e61727922222c0a20202020202020202222696d61676522223a222262616679626569686379727561657a613775796a64367567696362637271756d656a6636756633353365356574646b676f7471666677746775766122222c0a202020202020202022226d656469615479706522223a2222696d6167652f706e6722222c0a202020202020202022226465736372697074696f6e22223a22224d7920446573637269747074696f6e20666f722074686973204e465422222c0a2020202020202020222266696c657322223a5b0a2020202020202020202020207b0a2020202020202020202020202020202022226e616d6522223a22226d7966696c656e616d653122222c0a2020202020202020202020202020202022226d656469615479706522223a222266696c652f7a697022222c0a20202020202020202020202020202020222273726322223a205b22226d79536f75726365506174682f66696c652e7a697022225d0a2020202020202020202020207d2c7b0a2020202020202020202020202020202022226e616d6522223a22226d7966696c656e616d653222222c0a2020202020202020202020202020202022226d656469615479706522223a2222766964656f2f6d6f7622222c0a20202020202020202020202020202020222273726322223a205b22226d79536f75726365506174682f7669642e6d6f7622225d0a2020202020202020202020207d0a20202020202020205d2c0a202020202020202022226f7468657222223a222270726f7065727469657322222c0a2020202020202020222274726169747322223a5b222274726169743122222c222274726169743222222c222274726169743322222c222274726169743422222c222274726169743522222c222274726169743622222c222274726169743722222c222274726169743822225d0a202020207d0a7d220a";
        crate::api::import_nfts_from_csv_metadata(&hex::decode(csv).unwrap(), 0, 3)
            .await
            .unwrap();
        println!("Imported NFTs");
    }
}
/*
\metadata
"{
    ""myassetname_utf8"" : {
        ""name"":""myassetname_utf8"",
        ""image"":""bafybeihcyruaeza7uyjd6ugicbcrqumejf6uf353e5etdkhotqffwtguva"",
        ""mediaType"":""image/png"",
        ""description"":""My Descritption for this NFT"",
        ""files"":[
            {
                ""name"":""myfilename1"",
                ""mediaType"":""file/zip"",
                ""src"": [""mySourcePath/file.zip""]
            },{
                ""name"":""myfilename2"",
                ""mediaType"":""video/mov"",
                ""src"": [""mySourcePath/vid.mov""]
            }
        ],
        ""other"":""properties"",
        ""traits"":[""trait1"",""trait2"",""trait3"",""trait4""]
    },

    ""6d7961737365746e616d655f62696e617279"" : {
        ""name"":""myassetname_binary"",
        ""image"":""bafybeihcyruaeza7uyjd6ugicbcrqumejf6uf353e5etdkgotqffwtguva"",
        ""mediaType"":""image/png"",
        ""description"":""My Descritption for this NFT"",
        ""files"":[
            {
                ""name"":""myfilename1"",
                ""mediaType"":""file/zip"",
                ""src"": [""mySourcePath/file.zip""]
            },{
                ""name"":""myfilename2"",
                ""mediaType"":""video/mov"",
                ""src"": [""mySourcePath/vid.mov""]
            }
        ],
        ""other"":""properties"",
        ""traits"":[""trait1"",""trait2"",""trait3"",""trait4"",""trait5"",""trait6"",""trait7"",""trait8""]
    }
}"
*/
