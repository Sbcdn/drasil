/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/

use super::models::*;
use crate::error::RWDError;
use crate::schema::*;
use crate::*;
use async_recursion::async_recursion;
use chrono::{DateTime, Utc};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::sql_types::{Bool, Bytea, Int8, Nullable, Timestamptz, Varchar};

impl MintProject {
    pub fn get_mintproject_by_id(id_in: i64) -> Result<MintProject, RWDError> {
        let conn = &mut establish_connection()?;
        let result = mint_projects::table
            .filter(mint_projects::id.eq(id_in))
            .first::<MintProject>(conn)?;
        Ok(result)
    }

    pub fn get_mintproject_by_id_active(id_in: i64) -> Result<MintProject, RWDError> {
        let conn = &mut establish_connection()?;
        let result = mint_projects::table
            .filter(mint_projects::id.eq(id_in))
            .filter(mint_projects::active.eq(true))
            .first::<MintProject>(conn)?;
        Ok(result)
    }

    pub fn get_mintproject_by_uid_cid(uid_in: i64, cid_in: i64) -> Result<MintProject, RWDError> {
        let conn = &mut establish_connection()?;
        let result = mint_projects::table
            .filter(mint_projects::user_id.eq(uid_in))
            .filter(mint_projects::mint_contract_id.eq(cid_in))
            .first::<MintProject>(conn)?;
        Ok(result)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_mintproject<'a>(
        project_name: &'a String,
        user_id: &'a i64,
        mint_contract_id: &'a i64,
        whitelists: Option<&'a Vec<i64>>,
        mint_start_date: &'a DateTime<Utc>,
        mint_end_date: Option<&'a DateTime<Utc>>,
        storage_type: &'a String,
        storage_url: Option<&'a String>,
        storage_access_token: Option<&'a String>,
        collection_name: &'a String,
        author: &'a String,
        meta_description: &'a String,
        meta_common_nft_name: Option<&'a String>,
        max_mint_p_addr: Option<&'a i32>,
        nft_table_name: &'a String,
        active: &'a bool,
    ) -> Result<MintProject, RWDError> {
        let conn = &mut establish_connection()?;
        let new_entry = MintProjectNew {
            project_name,
            user_id,
            mint_contract_id,
            whitelists,
            mint_start_date,
            mint_end_date,
            storage_type,
            storage_url,
            storage_access_token,
            collection_name,
            author,
            meta_description,
            meta_common_nft_name,
            max_mint_p_addr,
            nft_table_name,
            active,
        };
        log::debug!("try to insert mint project into db...");
        let q = diesel::insert_into(mint_projects::table)
            .values(&new_entry)
            .get_result::<MintProject>(conn);
        println!("insert error: {q:?}");
        Ok(q?)
    }

    pub fn remove_mintproject(conn: &mut PgConnection, id_in: &i64) -> Result<usize, RWDError> {
        let result = diesel::delete(mint_projects::table.find(id_in)).execute(conn)?;

        Ok(result)
    }
}

/// Ugly Helper Type to manage the dynamic tables
type HNft<'a> = (
    diesel_dynamic_schema::Column<
        diesel_dynamic_schema::Table<&'a str, &'a str>,
        &'a str,
        diesel::sql_types::BigInt,
    >,
    diesel_dynamic_schema::Column<
        diesel_dynamic_schema::Table<&'a str, &'a str>,
        &'a str,
        diesel::sql_types::Binary,
    >,
    diesel_dynamic_schema::Column<
        diesel_dynamic_schema::Table<&'a str, &'a str>,
        &'a str,
        diesel::sql_types::Text,
    >,
    diesel_dynamic_schema::Column<
        diesel_dynamic_schema::Table<&'a str, &'a str>,
        &'a str,
        diesel::sql_types::Text,
    >,
    diesel_dynamic_schema::Column<
        diesel_dynamic_schema::Table<&'a str, &'a str>,
        &'a str,
        diesel::sql_types::Text,
    >,
    diesel_dynamic_schema::Column<
        diesel_dynamic_schema::Table<&'a str, &'a str>,
        &'a str,
        diesel::sql_types::Nullable<diesel::sql_types::Text>,
    >,
    diesel_dynamic_schema::Column<
        diesel_dynamic_schema::Table<&'a str, &'a str>,
        &'a str,
        diesel::sql_types::Nullable<diesel::sql_types::Text>,
    >,
    diesel_dynamic_schema::Column<
        diesel_dynamic_schema::Table<&'a str, &'a str>,
        &'a str,
        diesel::sql_types::Nullable<diesel::sql_types::Text>,
    >,
    diesel_dynamic_schema::Column<
        diesel_dynamic_schema::Table<&'a str, &'a str>,
        &'a str,
        diesel::sql_types::Nullable<diesel::sql_types::Text>,
    >,
    diesel_dynamic_schema::Column<
        diesel_dynamic_schema::Table<&'a str, &'a str>,
        &'a str,
        diesel::sql_types::Bool,
    >,
    diesel_dynamic_schema::Column<
        diesel_dynamic_schema::Table<&'a str, &'a str>,
        &'a str,
        diesel::sql_types::Nullable<diesel::sql_types::Text>,
    >,
    diesel_dynamic_schema::Column<
        diesel_dynamic_schema::Table<&'a str, &'a str>,
        &'a str,
        diesel::sql_types::Bool,
    >,
    diesel_dynamic_schema::Column<
        diesel_dynamic_schema::Table<&'a str, &'a str>,
        &'a str,
        diesel::sql_types::Timestamptz,
    >,
    diesel_dynamic_schema::Column<
        diesel_dynamic_schema::Table<&'a str, &'a str>,
        &'a str,
        diesel::sql_types::Timestamptz,
    >,
);

impl Nft {
    pub async fn db_client() -> Result<tokio_postgres::Client, RWDError> {
        let (client, connection) =
            tokio_postgres::connect(&std::env::var("REWARDS_DB_URL")?, tokio_postgres::NoTls)
                .await?;
        tokio::spawn(async move {
            if let Err(error) = connection.await {
                eprintln!("Connection error: {error}");
            }
        });
        Ok(client)
    }

    fn diesel_nft_table_definition(
        t: &'_ str,
    ) -> std::result::Result<(diesel_dynamic_schema::Table<&'_ str>, HNft), error::RWDError> {
        let table = diesel_dynamic_schema::table(t);
        let project_id = table.column::<Int8, _>("project_id");
        let asset_name_b = table.column::<Bytea, _>("asset_name_b");
        let asset_name = table.column::<Varchar, _>("asset_name");
        let fingerprint = table.column::<Varchar, _>("fingerprint");
        let nft_id = table.column::<Varchar, _>("nft_id");
        let file_name = table.column::<Nullable<Varchar>, _>("file_name");
        let ipfs_hash = table.column::<Nullable<Varchar>, _>("ipfs_hash");
        let metadata = table.column::<Nullable<Varchar>, _>("metadata");
        let claim_addr = table.column::<Nullable<Varchar>, _>("claim_addr");
        let minted = table.column::<Bool, _>("minted");
        let tx_hash = table.column::<Nullable<Varchar>, _>("tx_hash");
        let confirmed = table.column::<Bool, _>("confirmed");
        let created_at = table.column::<Timestamptz, _>("created_at");
        let updated_at = table.column::<Timestamptz, _>("updated_at");

        let t_clmns = (
            project_id,   //0
            asset_name_b, //1
            asset_name,   //2
            fingerprint,  //3
            nft_id,       //4
            file_name,    //5
            ipfs_hash,    //6
            metadata,     //7
            claim_addr,   //8
            minted,       //9
            tx_hash,      //10
            confirmed,    //11
            created_at,   //12
            updated_at,   //13
        );
        Ok((table, t_clmns))
    }

    pub fn get_nfts_by_pid(
        conn: &mut PgConnection,
        pid_in: i64,
        table: &str,
    ) -> Result<Vec<Nft>, RWDError> {
        let (table, clmns) = Nft::diesel_nft_table_definition(table)?;
        let result = table
            .select(clmns)
            .filter(clmns.0.eq(pid_in))
            .load::<Nft>(conn)?;
        Ok(result)
    }

    pub fn get_nft_by_assetnameb(
        pid_in: i64,
        table: &str,
        assetname_in: &Vec<u8>,
    ) -> Result<Nft, RWDError> {
        let conn = &mut establish_connection()?;
        let (table, clmns) = Nft::diesel_nft_table_definition(table)?;
        let result = table
            .select(clmns)
            .filter(clmns.0.eq(pid_in))
            .filter(clmns.1.eq(assetname_in))
            .first::<Nft>(conn)?;
        Ok(result)
    }

    pub fn get_nft_by_assetname_str(
        pid_in: i64,
        table: &str,
        assetname_in: &String,
    ) -> Result<Nft, RWDError> {
        let conn = &mut establish_connection()?;
        let (table, clmns) = Nft::diesel_nft_table_definition(table)?;
        let result = table
            .select(clmns)
            .filter(clmns.0.eq(pid_in))
            .filter(clmns.2.eq(assetname_in))
            .first::<Nft>(conn)?;
        Ok(result)
    }

    #[async_recursion]
    pub async fn claim_random_unminted_nft(
        pid_in: i64,
        table_name: &str,
        claim_addr_in: &String,
        tries: u8,
    ) -> Result<Option<Nft>, RWDError> {
        log::debug!("try to connect...");
        let (client, connection) =
            tokio_postgres::connect(&std::env::var("REWARDS_DB_URL")?, tokio_postgres::NoTls)
                .await?;
        // Spawn connection
        log::debug!("spawn connection...");
        tokio::spawn(async move {
            if let Err(error) = connection.await {
                eprintln!("Connection error: {error}");
            }
        });

        let rnd_nft_query = format!(
            "SELECT * FROM {table_name} TABLESAMPLE SYSTEM_ROWS(1) WHERE claim_addr IS NULL AND tx_hash IS NULL AND minted = false"
        );
        let mut rows = client.query(&rnd_nft_query, &[]).await?;
        log::debug!("First Select: {:?}", rows);
        let nft: Nft;
        if rows.is_empty() || rows.len() > 1 {
            let last_nft_query = format!(
                "SELECT * FROM {table_name} WHERE claim_addr IS NULL AND tx_hash IS NULL AND minted = false"
            );
            rows = client.query(&last_nft_query, &[]).await?;
            log::debug!("Second Select: {:?}", rows);
            match rows.len() {
                0 => {
                    log::error!("Couldn't find valid asset");
                    return Err(RWDError::new("Couldn't find valid nft to claim"));
                }
                _ => {
                    use rand::Rng;
                    let rnd = rand::thread_rng().gen_range(0..rows.len());
                    nft = Nft {
                        project_id: rows[rnd].get("project_id"),
                        asset_name_b: rows[rnd].get("asset_name_b"),
                        asset_name: rows[rnd].get("asset_name"),
                        fingerprint: rows[rnd].get("fingerprint"),
                        nft_id: rows[rnd].get("nft_id"),
                        file_name: rows[rnd].get("file_name"),
                        ipfs_hash: rows[rnd].get("ipfs_hash"),
                        metadata: rows[rnd].get("metadata"),
                        claim_addr: rows[rnd].get("claim_addr"),
                        minted: rows[rnd].get("minted"),
                        tx_hash: rows[rnd].get("tx_hash"),
                        confirmed: rows[rnd].get("confirmed"),
                        created_at: rows[rnd].get("created_at"),
                        updated_at: rows[rnd].get("updated_at"),
                    };
                }
            }
        } else {
            nft = Nft {
                project_id: rows[0].get("project_id"),
                asset_name_b: rows[0].get("asset_name_b"),
                asset_name: rows[0].get("asset_name"),
                fingerprint: rows[0].get("fingerprint"),
                nft_id: rows[0].get("nft_id"),
                file_name: rows[0].get("file_name"),
                ipfs_hash: rows[0].get("ipfs_hash"),
                metadata: rows[0].get("metadata"),
                claim_addr: rows[0].get("claim_addr"),
                minted: rows[0].get("minted"),
                tx_hash: rows[0].get("tx_hash"),
                confirmed: rows[0].get("confirmed"),
                created_at: rows[0].get("created_at"),
                updated_at: rows[0].get("updated_at"),
            };
        }
        println!("\nNFT: {nft:?}");

        Nft::set_nft_claim_addr(&pid_in, table_name, &nft.asset_name_b, claim_addr_in).await?;

        let check = format!(
            "SELECT * FROM {table_name} WHERE claim_addr = $1 AND tx_hash IS NULL AND minted = false"
        );

        let result = client.query(&check, &[claim_addr_in]).await;
        log::debug!("check: {:?}", rows);

        println!("{result:?}");
        match result {
            Ok(o) => {
                log::debug!("result: {:?}", o);
                return Ok(Some(nft));
            }
            Err(_) => {
                println!("Tries: {tries:?}");
                if tries < 5 {
                    return Nft::claim_random_unminted_nft(
                        pid_in,
                        table_name,
                        claim_addr_in,
                        tries + 1,
                    )
                    .await;
                }
            }
        }
        Err(RWDError::new("Could not claim an NFT"))
    }

    pub fn get_nft_by_claim_addr(
        pid_in: i64,
        claim_addr_in: &String,
        table: &str,
    ) -> Result<Vec<Nft>, RWDError> {
        let conn = &mut establish_connection()?;
        let (table, clmns) = Nft::diesel_nft_table_definition(table)?;
        let result = table
            .select(clmns)
            .filter(clmns.0.eq(pid_in))
            .filter(clmns.8.eq(claim_addr_in))
            .load::<Nft>(conn)?;
        Ok(result)
    }

    pub fn get_nft_by_claim_addr_unminted(
        pid_in: i64,
        claim_addr_in: &String,
        table: &str,
    ) -> Result<Vec<Nft>, RWDError> {
        let conn = &mut establish_connection()?;
        let (table, clmns) = Nft::diesel_nft_table_definition(table)?;
        let result = table
            .select(clmns)
            .filter(clmns.0.eq(pid_in))
            .filter(clmns.8.eq(claim_addr_in))
            .filter(clmns.9.eq(false))
            .load::<Nft>(conn)?;
        Ok(result)
    }

    pub fn get_all_claimed_unminted(pid_in: i64, table: &str) -> Result<Vec<Nft>, RWDError> {
        let conn = &mut establish_connection()?;
        let (table, clmns) = Nft::diesel_nft_table_definition(table)?;
        let result = table
            .select(clmns)
            .filter(clmns.0.eq(pid_in))
            .filter(clmns.8.is_not_null())
            .filter(clmns.9.eq(false))
            .load::<Nft>(conn)?;
        Ok(result)
    }

    pub fn get_all_minted(pid_in: i64, table: &str) -> Result<Vec<Nft>, RWDError> {
        let conn = &mut establish_connection()?;
        let (table, clmns) = Nft::diesel_nft_table_definition(table)?;
        let result = table
            .select(clmns)
            .filter(clmns.0.eq(pid_in))
            .filter(clmns.9.eq(true))
            .load::<Nft>(conn)?;
        Ok(result)
    }

    pub fn get_all_unconfirmed(pid_in: i64, table: &str) -> Result<Vec<Nft>, RWDError> {
        let conn = &mut establish_connection()?;
        let (table, clmns) = Nft::diesel_nft_table_definition(table)?;
        let result = table
            .select(clmns)
            .filter(clmns.0.eq(pid_in))
            .filter(clmns.9.eq(true))
            .filter(clmns.11.eq(false))
            .load::<Nft>(conn)?;
        Ok(result)
    }

    /*
                project_id,   //0
                asset_name_b, //1
                asset_name,   //2
                fingerprint,  //3
                nft_id,       //4
                file_name,    //5
                ipfs_hash,    //6
                metadata,     //7
                claim_addr,   //8
                minted,       //9
                tx_hash,      //10
                confirmed,    //11
                created_at,   //12
                updated_at,   //13

    */

    pub fn get_all_confirmed(pid_in: i64, table: &str) -> Result<Vec<Nft>, RWDError> {
        let conn = &mut establish_connection()?;
        let (table, clmns) = Nft::diesel_nft_table_definition(table)?;
        let result = table
            .select(clmns)
            .filter(clmns.0.eq(pid_in))
            .filter(clmns.9.eq(true))
            .filter(clmns.11.eq(true))
            .load::<Nft>(conn)?;
        Ok(result)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_nft<'a>(
        table_name: &'a str,
        project_id: &'a i64,
        asset_name_b: &'a Vec<u8>,
        asset_name: &'a String,
        fingerprint: &'a String,
        nft_id: &'a String,
        file_name: Option<&'a String>,
        ipfs_hash: Option<&'a String>,
        metadata: Option<&'a String>,
        claim_addr_in: Option<&'a String>,
    ) -> Result<Nft, RWDError> {
        log::debug!("try to connect...");
        let (client, connection) =
            tokio_postgres::connect(&std::env::var("REWARDS_DB_URL")?, tokio_postgres::NoTls)
                .await?;
        // Spawn connection
        log::debug!("spawn connection...");
        tokio::spawn(async move {
            if let Err(error) = connection.await {
                eprintln!("Connection error: {error}");
            }
        });

        let insert_query = format!("INSERT INTO {table_name} 
        (project_id, asset_name_b, asset_name, fingerprint, nft_id, file_name, ipfs_hash, metadata, claim_addr, minted, confirmed) 
        VALUES 
        ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)");

        let q = client
            .query(
                &insert_query,
                &[
                    project_id,
                    asset_name_b,
                    asset_name,
                    fingerprint,
                    nft_id,
                    &file_name,
                    &ipfs_hash,
                    &metadata,
                    &claim_addr_in,
                    &false,
                    &false,
                ],
            )
            .await;

        if q.is_err() {
            log::debug!("NFT existed already or another error {:?}", q)
        }

        let nft = Nft::get_nft_by_assetnameb(*project_id, table_name, asset_name_b)?;

        Ok(nft)
    }

    pub async fn set_nft_minted<'a>(
        pid_in: &'a i64,
        table_name: &'a str,
        fingerprint: &'a String,
        txhash_in: &'a String,
    ) -> Result<(), RWDError> {
        let client = Nft::db_client().await?;
        let update_query = format!(
            "UPDATE {table_name} SET minted = true, tx_hash=$1 WHERE project_id=$2 AND fingerprint=$3 AND minted is false AND tx_hash is Null", 
        );
        let rows = client
            .query(&update_query, &[txhash_in, &pid_in, fingerprint])
            .await?;
        if !rows.is_empty() {
            return Err(RWDError::new(&format!(
                "Could not set NFT: {fingerprint} to minted = true"
            )));
        }
        Ok(())
    }

    pub async fn set_nft_confirmed<'a>(
        pid_in: &'a i64,
        table_name: &str,
        fingerprint: &'a String,
        txhash_in: &'a String,
    ) -> Result<(), RWDError> {
        let client = Nft::db_client().await?;
        let update_query = format!(
            "UPDATE {table_name} SET confirmed = true WHERE project_id=$1 AND fingerprint=$2 AND minted = true AND tx_hash=$3", 
        );
        let rows = client
            .query(&update_query, &[pid_in, fingerprint, txhash_in])
            .await?;
        if !rows.is_empty() {
            return Err(RWDError::new(&format!(
                "Could not set NFT: {fingerprint} to confirmed = true"
            )));
        }
        Ok(())
    }

    pub async fn set_nft_claim_addr<'a>(
        pid_in: &'a i64,
        table_name: &str,
        asset_name_b: &'a Vec<u8>,
        claim_addr: &'a String,
    ) -> Result<(), RWDError> {
        let client = Nft::db_client().await?;
        let update_query = format!(
            "UPDATE {table_name} SET claim_addr = $1 WHERE project_id=$2 AND asset_name_b=$3 AND minted = false AND tx_hash IS NULL", 
        );
        let rows = client
            .query(&update_query, &[claim_addr, &pid_in, asset_name_b])
            .await?;
        if !rows.is_empty() {
            return Err(RWDError::new(&format!(
                "Could not set claim addr for NFT: {}",
                hex::encode(asset_name_b)
            )));
        }
        Ok(())
    }

    pub async fn set_nft_ipfs<'a>(
        pid_in: &'a i64,
        table_name: &str,
        asset_name_b: &'a Vec<u8>,
        ipfs_hash: &'a String,
    ) -> Result<(), RWDError> {
        let client = Nft::db_client().await?;
        let update_query = format!(
            "UPDATE {table_name} SET ipfs_hash = $1 WHERE project_id=$2 AND asset_name_b=$3 AND minted = false", 
        );

        let rows = client
            .query(&update_query, &[ipfs_hash, &pid_in, &asset_name_b])
            .await?;
        if !rows.is_empty() {
            return Err(RWDError::new(&format!(
                "Could not set ipfs hash for NFT: {}",
                hex::encode(asset_name_b)
            )));
        }
        Ok(())
    }

    pub async fn set_nft_metadata<'a>(
        pid_in: &'a i64,
        table_name: &str,
        asset_name_b: &'a Vec<u8>,
        metadata: &'a String,
    ) -> Result<(), RWDError> {
        let client = Nft::db_client().await?;
        let update_query = format!(
            "UPDATE {table_name} SET metadata = $1 WHERE project_id=$2 AND asset_name_b=$3 AND minted = false", 
        );
        let rows = client
            .query(&update_query, &[metadata, &pid_in, asset_name_b])
            .await?;
        if !rows.is_empty() {
            return Err(RWDError::new(&format!(
                "Could not set metadata for NFT: {}",
                hex::encode(asset_name_b)
            )));
        }
        Ok(())
    }

    /*
    TODO:
      On Mint-Project creation a user needs to set if a mint project will contain double IPFS images / filenames.
      This means an IPFS link / image / file can occur for more than one NFT.
      For this case the database constraints which block the doublets needs to be excluded from the creation query.
      For this case the drasil system can obviously not ensure that an NFT is uniquly minted.
      The same might apply if you want Semi-Fungible Tokens but that needs anyway additional concepts.
    */
    pub async fn create_nft_table(str: &String) -> Result<(), RWDError> {
        log::debug!("try to connect...");
        let (client, connection) =
            tokio_postgres::connect(&std::env::var("REWARDS_DB_URL")?, tokio_postgres::NoTls)
                .await?;
        // Spawn connection
        log::debug!("spawn connection...");
        tokio::spawn(async move {
            if let Err(error) = connection.await {
                eprintln!("Connection error: {error}");
            }
        });

        log::debug!("try to create table...");
        let create_str = format!(
            "CREATE TABLE IF NOT EXISTS {str} (
            project_id BIGINT NOT NULL,
            asset_name_b BYTEA PRIMARY KEY,
            asset_name VARCHAR NOT NULL,
            fingerprint VARCHAR NOT NULL UNIQUE,
            nft_id VARCHAR NOT NULL UNIQUE,
            file_name VARCHAR NOT NULL UNIQUE,
            ipfs_hash VARCHAR UNIQUE,
            metadata TEXT NOT NULL,
            claim_addr VARCHAR,
            minted BOOLEAN NOT NULL,
            tx_hash VARCHAR,
            confirmed BOOLEAN,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW());
            CREATE TRIGGER set_timestamp
            BEFORE UPDATE ON {str}
            FOR EACH ROW
            EXECUTE PROCEDURE trigger_set_timestamp();"
        ); //DROP TRIGGER IF EXISTS set_timestamp ON {};

        client.batch_execute(&create_str).await?;

        Ok(())
    }
}

impl MintReward {
    pub fn get_mintrewards_by_pid_addr(
        pid_in: i64,
        pay_addr_in: &String,
    ) -> Result<Vec<MintReward>, RWDError> {
        let result = mint_rewards::table
            .filter(mint_rewards::pay_addr.eq(pay_addr_in))
            .filter(mint_rewards::project_id.eq(pid_in))
            .load::<MintReward>(&mut establish_connection()?)?;
        Ok(result)
    }

    pub fn get_avail_mintrewards_by_addr(
        pay_addr_in: &String,
    ) -> Result<Vec<MintReward>, RWDError> {
        let result = mint_rewards::table
            .filter(mint_rewards::pay_addr.eq(pay_addr_in))
            .filter(mint_rewards::processed.eq(false))
            .filter(mint_rewards::minted.eq(false))
            .load::<MintReward>(&mut establish_connection()?)?;
        Ok(result)
    }

    pub fn get_avail_mintrewards_cl_by_addr(
        user_id_in: i64,
        pay_addr_in: &String,
    ) -> Result<Vec<MintReward>, RWDError> {
        let projects = mint_projects::table
            .filter(mint_projects::user_id.eq(user_id_in))
            .load::<MintProject>(&mut establish_connection()?)?;
        let mut mintrewards = Vec::<MintReward>::new();
        for p in projects {
            mintrewards.extend(
                mint_rewards::table
                    .filter(mint_rewards::pay_addr.eq(pay_addr_in))
                    .filter(mint_rewards::project_id.eq(p.id))
                    .filter(mint_rewards::processed.eq(false))
                    .filter(mint_rewards::minted.eq(false))
                    .load::<MintReward>(&mut establish_connection()?)?
                    .into_iter(),
            )
        }
        Ok(mintrewards)
    }

    pub fn get_mintreward_by_id(id_in: i64) -> Result<MintReward, RWDError> {
        let result = mint_rewards::table
            .filter(mint_rewards::id.eq(id_in))
            .first::<MintReward>(&mut establish_connection()?)?;
        Ok(result)
    }

    pub fn get_mintreward_by_nft_ids(ids_in: Vec<Vec<u8>>) -> Result<MintReward, RWDError> {
        let result = mint_rewards::table
            .filter(mint_rewards::nft_ids.eq(ids_in))
            .first::<MintReward>(&mut establish_connection()?)?;
        Ok(result)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_mintreward<'a>(
        user_id: i64,
        contract_id: i64,
        pay_addr: &'a String,
        nft_ids: Vec<&'a Vec<u8>>,
        v_nfts_b: Vec<&'a Vec<u8>>, // serialized clib::utils::Value
    ) -> Result<MintReward, RWDError> {
        let conn = &mut establish_connection()?;

        let mint_project = MintProject::get_mintproject_by_uid_cid(user_id, contract_id)?;

        let new_entry = MintRewardNew {
            project_id: &mint_project.id,
            pay_addr,
            nft_ids,
            v_nfts_b,
            processed: &false,
            minted: &false,
        };
        log::debug!("try to insert mint reward into db...");
        let q = diesel::insert_into(mint_rewards::table)
            .values(&new_entry)
            .get_result::<MintReward>(conn);
        println!("insert error?: {q:?}");
        Ok(q?)
    }

    pub fn update_payaddr(id_in: i64, pay_addr_in: &String) -> Result<MintReward, RWDError> {
        log::debug!("try to update payaddr on mint reward...");
        let conn = &mut establish_connection()?;
        let mintreward = diesel::update(
            mint_rewards::table
                .filter(mint_rewards::id.eq(id_in))
                .filter(mint_rewards::processed.eq(false))
                .filter(mint_rewards::minted.eq(false)),
        )
        .set((mint_rewards::pay_addr.eq(pay_addr_in),))
        .get_result::<MintReward>(conn)?;
        Ok(mintreward)
    }

    pub fn process_mintreward(
        id_in: i64,
        pid_in: i64,
        pay_addr_in: &String,
    ) -> Result<MintReward, RWDError> {
        let conn = &mut establish_connection()?;
        let mint_reward = MintReward::get_mintreward_by_id(id_in)?;

        if mint_reward.pay_addr != *pay_addr_in
            || mint_reward.processed
            || mint_reward.minted
            || mint_reward.project_id != pid_in
        {
            return Err(RWDError::new("The provided mintreward is invalid"));
        }
        let mp = MintProject::get_mintproject_by_id(mint_reward.project_id)?;
        for nft_id in &mint_reward.nft_ids {
            let nft =
                Nft::get_nft_by_assetnameb(mint_reward.project_id, &mp.nft_table_name, nft_id)?;
            if nft.claim_addr.unwrap() != *pay_addr_in || nft.minted {
                return Err(RWDError::new("invalid minting request"));
            }
        }
        let mintreward = diesel::update(mint_rewards::table.filter(mint_rewards::id.eq(id_in)))
            .set((mint_rewards::processed.eq(true),))
            .get_result::<MintReward>(conn)?;
        Ok(mintreward)
    }

    pub fn mint_mintreward(
        id_in: i64,
        pid_in: i64,
        pay_addr_in: &String,
    ) -> Result<MintReward, RWDError> {
        let conn = &mut establish_connection()?;
        let mint_reward = MintReward::get_mintreward_by_id(id_in)?;

        if mint_reward.pay_addr != *pay_addr_in
            || !mint_reward.processed
            || mint_reward.minted
            || mint_reward.project_id != pid_in
        {
            return Err(RWDError::new("The provided mintreward is invalid"));
        }
        let mintreward = diesel::update(mint_rewards::table.filter(mint_rewards::id.eq(id_in)))
            .set((mint_rewards::minted.eq(true),))
            .get_result::<MintReward>(conn)?;
        Ok(mintreward)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_nft_functions() {
        std::env::set_var(
            "REWARDS_DB_URL",
            "", // set db connection string
        );
        let gcon = &mut establish_connection().unwrap();

        MintProject::remove_mintproject(gcon, &99991).unwrap();
        log::debug!("try to connect...");
        let client = Nft::db_client().await.unwrap();
        let drop = client
            .batch_execute("DROP TABLE IF EXISTS test_table;")
            .await;
        match drop {
            Ok(_) => (),
            Err(e) => {
                eprintln!("Drop error: {e}");
            }
        }

        MintProject::create_mintproject(
            &"TestProject".to_string(),
            &0,
            &99,
            None,
            &Utc::now(),
            None,
            &"CIP25Metadata".to_string(),
            None,
            None,
            &"Test Collection".to_string(),
            &"Test0r".to_string(),
            &"A Test Project".to_string(),
            Some(&"test".to_string()),
            Some(&1),
            &"test_table".to_string(),
            &true,
        )
        .unwrap();

        models::Nft::create_nft_table(&"test_table".to_string())
            .await
            .unwrap();

        let nft = Nft::create_nft(
            "test_table",
            &99991,
            &"MyAsset1".to_string().as_bytes().to_vec(),
            &"MyAsset1".to_string(),
            &"fingerprint".to_string(),
            &"0001".to_string(),
            Some(&"MyAsset10001.png".to_string()),
            None,
            Some(&"MyAsset10001.png".to_string()),
            None,
        )
        .await
        .unwrap();

        let claim1 = minting::models::Nft::claim_random_unminted_nft(
            99991,
            "test_table",
            &"addr_test14twfas9fui0sa9dfu0xduv0xuv0xuv0du0x9uvd09ux".to_string(),
            0,
        )
        .await;

        assert_eq!(claim1, Ok(Some(nft)));

        let claim2 = minting::models::Nft::claim_random_unminted_nft(
            99991,
            "test_table",
            &"addr_test1f487487487f897498f74n998498".to_string(),
            0,
        )
        .await;
        match claim2 {
            Ok(Some(nft)) => {
                panic!("second claim should fail, claimed NFT: {nft:?}");
            }
            Ok(x) => {
                assert_eq!(x, None);
            }
            Err(e) => {
                println!("Error: {e}");
            }
        }
        let claims = Nft::get_all_claimed_unminted(99991, "test_table").unwrap();
        println!("Claims: {claims:?}");

        let _ = Nft::create_nft(
            "test_table",
            &99991,
            &"MyAsset2".to_string().as_bytes().to_vec(),
            &"MyAsset2".to_string(),
            &"asset19chwwkp2pftlxdsxszv6edyj070j4nvhah5ugd".to_string(),
            &"0002".to_string(),
            Some(&"MyAsset20002.png".to_string()),
            None,
            Some(&"MyAsset20002.png".to_string()),
            None,
        )
        .await
        .unwrap();

        let all_claimed = Nft::get_all_claimed_unminted(99991, "test_table").unwrap();
        assert_eq!(all_claimed.len(), 1);
        let all_minted = Nft::get_all_minted(99991, "test_table").unwrap();
        assert_eq!(all_minted.len(), 0);
        let all_unconfirmed = Nft::get_all_unconfirmed(99991, "test_table").unwrap();
        assert_eq!(all_unconfirmed.len(), 0);
        let all_confirmed = Nft::get_all_confirmed(99991, "test_table").unwrap();
        assert_eq!(all_confirmed.len(), 0);

        Nft::set_nft_ipfs(
            &99991,
            "test_table",
            &"MyAsset2".to_string().as_bytes().to_vec(),
            &"bafybeihcyruaeza8uyjd6ugfcbcrqumejf6uf353e5etdkhotqffwtguva".to_string(),
        )
        .await
        .unwrap();

        let nft_ = Nft::get_nft_by_assetnameb(
            99991,
            "test_table",
            &"MyAsset2".to_string().as_bytes().to_vec(),
        )
        .unwrap();
        assert_eq!(
            nft_.ipfs_hash.unwrap(),
            "bafybeihcyruaeza8uyjd6ugfcbcrqumejf6uf353e5etdkhotqffwtguva"
        );

        Nft::set_nft_metadata(
            &99991,
            "test_table",
            &"MyAsset2".to_string().as_bytes().to_vec(),
            &"{\"name\":\"This is Token1\"}".to_string(),
        )
        .await
        .unwrap();

        let nft_ =
            Nft::get_nft_by_assetname_str(99991, "test_table", &"MyAsset2".to_string()).unwrap();
        assert_eq!(
            nft_.metadata.unwrap(),
            "{\"name\":\"This is Token1\"}".to_string()
        );

        Nft::set_nft_claim_addr(
            &99991,
            "test_table",
            &"MyAsset2".to_string().as_bytes().to_vec(),
            &"addr_test1wpqkdeh52adpqf57n83xhaze4gkzr9u2mfwa23lcfnpgdzs72t77u".to_string(),
        )
        .await
        .unwrap();

        let nft_ = Nft::get_nft_by_claim_addr(
            99991,
            &"addr_test1wpqkdeh52adpqf57n83xhaze4gkzr9u2mfwa23lcfnpgdzs72t77u".to_string(),
            "test_table",
        )
        .unwrap();
        assert_eq!(
            nft_[0].clone().ipfs_hash.unwrap(),
            "bafybeihcyruaeza8uyjd6ugfcbcrqumejf6uf353e5etdkhotqffwtguva"
        );

        let all_claimed = Nft::get_all_claimed_unminted(99991, "test_table").unwrap();
        assert_eq!(all_claimed.len(), 2);

        Nft::set_nft_minted(
            &99991,
            "test_table",
            &"asset19chwwkp2pftlxdsxszv6edyj070j4nvhah5ugd".to_string(),
            &"3ac4574762c5ed6bfd7a6e4e23759bff0a8febd679f1316e232ff080134dae2f".to_string(),
        )
        .await
        .unwrap();

        let all_minted = Nft::get_all_minted(99991, "test_table").unwrap();
        assert_eq!(all_minted.len(), 1);

        let all_unconfirmed = Nft::get_all_unconfirmed(99991, "test_table").unwrap();
        assert_eq!(all_unconfirmed.len(), 1);

        Nft::set_nft_confirmed(
            &99991,
            "test_table",
            &"asset19chwwkp2pftlxdsxszv6edyj070j4nvhah5ugd".to_string(),
            &"3ac4574762c5ed6bfd7a6e4e23759bff0a8febd679f1316e232ff080134dae2f".to_string(),
        )
        .await
        .unwrap();

        let all_confirmed = Nft::get_all_confirmed(99991, "test_table").unwrap();
        assert_eq!(all_confirmed.len(), 1);
    }
}
