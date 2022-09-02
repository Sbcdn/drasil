/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use super::*;
use crate::{
    admin::get_vaddr,
    encryption::{decrypt, encrypt, vault_get, vault_store},
    schema::{contracts, email_verification_token, multisig_keyloc},
};
use diesel::pg::upsert::on_constraint;
use error::SystemDBError;
use murin::{
    crypto::{Ed25519Signature, PrivateKey, PublicKey},
    get_network_from_address, TransactionUnspentOutputs, TxData,
};
use sha2::Digest;

impl TBContracts {
    pub fn get_liquidity_wallet(user_id_in: &i64) -> Result<TBContracts, SystemDBError> {
        use crate::schema::contracts::dsl::*;
        let result = contracts
            .filter(
                contract_type
                    .eq(&crate::datamodel::hephadata::ContractType::DrasilAPILiquidity.to_string()),
            )
            .filter(user_id.eq(user_id_in))
            .first::<TBContracts>(&mut establish_connection()?)?;
        Ok(result)
    }

    pub fn get_contract_liquidity(&self) -> murin::utils::BigNum {
        let e = self.external_lqdty.unwrap_or(0);
        let d = self.drasil_lqdty.unwrap_or(0);
        let o = self.customer_lqdty.unwrap_or(0);
        murin::utils::to_bignum((e + d + o) as u64)
    }

    pub fn get_contract_for_user(
        conn: &mut PgConnection,
        uid: i64,
        ctype: String,
        vers: Option<f32>,
    ) -> Result<TBContracts, SystemDBError> {
        use crate::schema::contracts::dsl::*;
        let result = contracts
            .filter(user_id.eq(&uid))
            .filter(contract_type.eq(&ctype))
            .order(version.desc())
            .load::<TBContracts>(conn)?;

        let err = SystemDBError::Custom(format!(
            "no contract found for user-id: '{}' and contract type '{}'",
            uid, ctype
        ));

        if let Some(v) = vers {
            let result: Vec<TBContracts> = result
                .into_iter()
                .filter(|elem| elem.version == v)
                .collect();
            if let Some(r) = result.get(0) {
                return Ok(r.clone());
            };
        } else {
            // get latest Version
            if let Some(r) = result.get(0) {
                return Ok(r.clone());
            };
        }
        Err(err)
    }

    pub fn get_active_contract_for_user(
        conn: &mut PgConnection,
        uid: i64,
        ctype: String,
        vers: Option<f32>,
    ) -> Result<TBContracts, SystemDBError> {
        use crate::schema::contracts::dsl::*;
        let result = contracts
            .filter(user_id.eq(&uid))
            .filter(contract_type.eq(&ctype))
            .filter(depricated.eq(false))
            .order(version.desc())
            .load::<TBContracts>(conn)?;

        let err = SystemDBError::Custom(format!(
            "no contract found for user-id: '{}' and contract type '{}'",
            uid, ctype
        ));

        if let Some(v) = vers {
            let result: Vec<TBContracts> = result
                .into_iter()
                .filter(|elem| elem.version == v)
                .collect();
            if let Some(r) = result.get(0) {
                return Ok(r.clone());
            };
        } else {
            // get latest Version
            if let Some(r) = result.get(0) {
                return Ok(r.clone());
            };
        }
        Err(err)
    }

    pub fn get_all_contracts_for_user_typed(
        uid: i64,
        ctype: String,
    ) -> Result<Vec<TBContracts>, SystemDBError> {
        use crate::schema::contracts::dsl::*;

        let result = contracts
            .filter(user_id.eq(&(uid)))
            .filter(contract_type.eq(&ctype))
            .order(contract_id.asc())
            .load::<TBContracts>(&mut establish_connection()?)?;

        Ok(result)
    }

    pub fn get_all_contracts_for_user(uid: i64) -> Result<Vec<TBContracts>, SystemDBError> {
        use crate::schema::contracts::dsl::*;

        let result = contracts
            .filter(user_id.eq(&(uid)))
            .filter(depricated.eq(false))
            .order(contract_id.asc())
            .load::<TBContracts>(&mut establish_connection()?)?;

        Ok(result)
    }

    pub fn get_contract_uid_cid(
        user_id_in: i64,
        contract_id_in: i64,
    ) -> Result<TBContracts, SystemDBError> {
        log::debug!("try to get data from contracts table: ");
        let result = contracts::table
            .filter(contracts::user_id.eq(&user_id_in))
            .filter(contracts::contract_id.eq(&contract_id_in))
            .load::<TBContracts>(&mut establish_connection()?);

        log::debug!("Result: {:?}", result);

        Ok(result?[0].clone())
    }

    pub fn get_next_contract_id(user_id_in: &i64) -> Result<i64, SystemDBError> {
        use crate::schema::contracts::dsl::*;
        let result = contracts
            .filter(user_id.eq(user_id_in))
            .select(contract_id)
            .order(contract_id.desc())
            .limit(1)
            .load::<i64>(&mut establish_connection()?)?;

        let mut contract_id_new = 0;
        if !result.is_empty() {
            contract_id_new = result[0] + 1;
        }
        Ok(contract_id_new)
    }

    pub fn get_contract_by_id(
        conn: &mut PgConnection,
        id_in: i64,
    ) -> Result<TBContracts, SystemDBError> {
        use crate::schema::contracts::dsl::*;
        let result = contracts.find(id_in).first::<TBContracts>(conn)?;
        Ok(result)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_contract<'a>(
        user_id: &'a i64,
        contract_id: &'a i64,
        contract_type: &'a str,
        description: Option<&'a str>,
        version: &'a f32,
        plutus: &'a str,
        address: &'a str,
        policy_id: Option<&'a String>,
        depricated: &'a bool,
    ) -> Result<TBContracts, SystemDBError> {
        let new_contract = TBContractNew {
            user_id,
            contract_id,
            contract_type,
            description,
            version,
            plutus,
            address,
            policy_id,
            depricated,
            drasil_lqdty: None,
            customer_lqdty: None,
            external_lqdty: None,
        };

        Ok(diesel::insert_into(contracts::table)
            .values(&new_contract)
            .get_result::<TBContracts>(&mut establish_connection()?)?)
    }

    pub fn update_contract<'a>(
        conn: &mut PgConnection,
        id_in: &'a i64,
        contract_id_new: &'a i64,
        description_new: Option<&'a str>,
        depricated_new: &'a bool,
    ) -> Result<TBContracts, SystemDBError> {
        use crate::schema::contracts::dsl::*;
        let contract = diesel::update(contracts.find(id_in))
            .set((
                contract_id.eq(contract_id_new),
                description.eq(description_new),
                depricated.eq(depricated_new),
            ))
            .get_result::<TBContracts>(conn)?;

        Ok(contract)
    }

    pub fn depricate_contract<'a>(
        conn: &mut PgConnection,
        user_id_in: &'a i64,
        contract_id_in: &'a i64,
        depricated_in: &'a bool,
    ) -> Result<TBContracts, SystemDBError> {
        use crate::schema::contracts::dsl::*;
        let contract = diesel::update(
            contracts
                .filter(user_id.eq(user_id_in))
                .filter(contract_id.eq(contract_id_in)),
        )
        .set(depricated.eq(depricated_in))
        .get_result::<TBContracts>(conn)?;
        Ok(contract)
    }
}

impl TBMultiSigLoc {
    #[allow(clippy::too_many_arguments)]
    pub async fn create_multisig_keyloc<'a>(
        user_id: &'a i64,
        contract_id: &'a i64,
        version: &'a f32,
        ca: &'a String,
        fee_wallet_addr: Option<&'a String>,
        fee: Option<&'a i64>,
        pvks: &'a [String],
        depricated: &'a bool,
    ) -> Result<TBMultiSigLoc, SystemDBError> {
        let ident = crate::encryption::mident(user_id, contract_id, version, ca);
        let epvks = crate::encryption::encrypt_pvks(pvks, &ident).await?;
        let new_keyloc = TBMultiSigLocNew {
            user_id,
            contract_id,
            version,
            fee_wallet_addr,
            fee,
            pvks: &epvks,
            depricated,
        };
        //let conn = establish_connection()?;
        Ok(diesel::insert_into(multisig_keyloc::table)
            .values(&new_keyloc)
            .get_result::<TBMultiSigLoc>(&mut establish_connection()?)?)
    }

    pub fn get_multisig_keyloc(
        conn: &mut PgConnection,
        contract_id_in: &i64,
        user_id_in: &i64,
        version_in: &f32,
    ) -> Result<TBMultiSigLoc, SystemDBError> {
        use crate::schema::multisig_keyloc::dsl::*;
        let result = multisig_keyloc
            .filter(contract_id.eq(&contract_id_in))
            .filter(user_id.eq(&user_id_in))
            .filter(version.eq(&version_in))
            .load::<TBMultiSigLoc>(conn)?;

        let err = SystemDBError::Custom(format!("no multisig key location found for contract-id: '{}' User-id: '{}'  , version: '{}'; \n Result: {:?}"
                ,contract_id_in, user_id_in, version_in, result));

        if let Some(r) = result.get(0) {
            return Ok(r.clone());
        };

        Err(err)
    }
}

impl TBDrasilUser {
    fn get_next_user_id(conn: &mut PgConnection) -> Result<i64, SystemDBError> {
        use crate::schema::drasil_user::dsl::*;
        let result = drasil_user
            .select(user_id)
            .order_by(user_id.desc())
            .first::<i64>(conn)?;
        Ok(result + 1)
    }

    fn get_user_by_mail(
        conn: &mut PgConnection,
        email_in: &String,
    ) -> Result<TBDrasilUser, SystemDBError> {
        use crate::schema::drasil_user::dsl::*;
        let result = drasil_user
            .filter(email.eq(email_in))
            .first::<TBDrasilUser>(conn)?;
        Ok(result)
    }

    pub fn get_user_by_user_id(
        conn: &mut PgConnection,
        user_id_in: &i64,
    ) -> Result<TBDrasilUser, SystemDBError> {
        use crate::schema::drasil_user::dsl::*;
        let result = drasil_user
            .filter(user_id.eq(user_id_in))
            .first::<TBDrasilUser>(conn)?;
        Ok(result)
    }

    pub fn verify_pw_user(email: &String, pwd: &String) -> Result<TBDrasilUser, SystemDBError> {
        use argon2::{
            password_hash::{PasswordHash, PasswordVerifier},
            Argon2,
        };
        let user = TBDrasilUser::get_user_by_mail(&mut establish_connection()?, email)?;
        Argon2::default().verify_password(pwd.as_bytes(), &PasswordHash::new(&user.pwd)?)?;
        Ok(user)
    }

    pub fn verify_pw_userid(user_id: &i64, pwd: &String) -> Result<TBDrasilUser, SystemDBError> {
        use argon2::{
            password_hash::{PasswordHash, PasswordVerifier},
            Argon2,
        };
        let user = TBDrasilUser::get_user_by_user_id(&mut establish_connection()?, user_id)?;
        Argon2::default().verify_password(pwd.as_bytes(), &PasswordHash::new(&user.pwd)?)?;
        Ok(user)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_user<'a>(
        api_pubkey: Option<&'a String>,
        uname: &'a String,
        email: &'a String,
        pwd: &'a String,
        permissions: &'a Vec<String>,
        company_name: Option<&'a String>,
        address: Option<&'a String>,
        post_code: Option<&'a String>,
        city: Option<&'a String>,
        addional_addr: Option<&'a String>,
        country: Option<&'a String>,
        contact_p_fname: Option<&'a String>,
        contact_p_sname: Option<&'a String>,
        contact_p_tname: Option<&'a String>,
        identification: &'a Vec<String>,
        cardano_wallet: Option<&'a String>,
    ) -> Result<TBDrasilUser, SystemDBError> {
        log::debug!("create user");
        use argon2::{
            password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
            Argon2,
        };
        let mut conn = establish_connection()?;
        let nuser_id = TBDrasilUser::get_next_user_id(&mut conn);
        let user_id = match nuser_id {
            Ok(id) => id,
            Err(e) => {
                log::error!("Error did not found any userid");
                if e.to_string() == *"NotFound" {
                    0
                } else {
                    return Err(e);
                }
            }
        };
        let password_hash = Argon2::default()
            .hash_password(pwd.as_bytes(), &SaltString::generate(&mut OsRng))?
            .to_string();

        let (privkey, pubkey, pubkeyhash) = murin::wallet::create_drslkeypair();
        let privkey = encrypt(&privkey, pwd)?;
        vault_store(&pubkeyhash, "prvkey", &privkey).await;

        let new_user = TBDrasilUserNew {
            user_id: &user_id,
            api_pubkey,
            uname,
            email,
            pwd: &password_hash,
            role: &"3".to_string(),
            permissions,
            company_name,
            address,
            post_code,
            city,
            addional_addr,
            country,
            contact_p_fname,
            contact_p_sname,
            contact_p_tname,
            identification,
            email_verified: &false,
            cardano_wallet,
            cwallet_verified: &false,
            drslpubkey: &pubkey,
        };
        let user = match TBDrasilUser::get_user_by_mail(&mut conn, email) {
            Ok(u) => {
                if u.email_verified {
                    return Err(SystemDBError::Custom(
                        "User exists and is verified already".to_string(),
                    ));
                }
                u
            }
            Err(_e) => diesel::insert_into(drasil_user::table)
                .values(&new_user)
                .on_conflict(on_constraint("unique_email"))
                .do_nothing()
                .get_result::<TBDrasilUser>(&mut conn)?,
        };
        Ok(user)
    }

    pub fn verify_email(email_in: &String) -> Result<TBDrasilUser, SystemDBError> {
        use crate::schema::drasil_user::dsl::*;
        let mut conn = establish_connection()?;
        let user = TBDrasilUser::get_user_by_mail(&mut conn, email_in)?;

        let user_updated = diesel::update(drasil_user.find(user.id))
            .set((email_verified.eq(true),))
            .get_result::<TBDrasilUser>(&mut conn)?;

        Ok(user_updated)
    }

    pub fn update_api_key<'a>(
        user_id_in: &'a i64,
        token: &'a String,
    ) -> Result<TBDrasilUser, SystemDBError> {
        use crate::schema::drasil_user::dsl::*;
        let user_updated = diesel::update(drasil_user.find(user_id_in))
            .set((api_pubkey.eq(Some(token)),))
            .get_result::<TBDrasilUser>(&mut establish_connection()?)?;

        Ok(user_updated)
    }

    pub async fn approve(&self, pw: &String, msg: &str) -> Result<String, SystemDBError> {
        let pk = PublicKey::from_bech32(&self.drslpubkey)?;
        let pkh = hex::encode(PublicKey::from_bech32(&self.drslpubkey)?.hash().to_bytes());
        let privkey = vault_get(&pkh).await;
        let privkey = PrivateKey::from_bech32(&decrypt(privkey.get("prvkey").unwrap(), pw)?)?;

        let sign = privkey.sign(msg.as_bytes());
        let verify = pk.verify(msg.as_bytes(), &sign);
        assert!(verify);
        Ok(sign.to_hex())
    }

    pub fn verify_approval(&self, msg: &str, sign: &str) -> Result<bool, SystemDBError> {
        let pk = PublicKey::from_bech32(&self.drslpubkey)?;
        let sign = Ed25519Signature::from_hex(sign)?;
        if !pk.verify(msg.as_bytes(), &sign) {
            return Err(SystemDBError::Custom("Error: US0010".to_string()));
        }
        Ok(true)
    }
}

impl TBEmailVerificationToken {
    pub fn find(id: &Vec<u8>) -> Result<Self, SystemDBError> {
        let token = email_verification_token::table
            .filter(email_verification_token::id.eq(id))
            .first(&mut establish_connection()?)?;

        Ok(token)
    }

    pub fn find_by_mail(email_in: &str) -> Result<Self, SystemDBError> {
        let token = email_verification_token::table
            .filter(email_verification_token::email.eq(email_in))
            .first(&mut establish_connection()?)?;

        Ok(token)
    }

    pub fn create(body: TBEmailVerificationTokenMessage) -> Result<Self, SystemDBError> {
        use rand::Rng;

        let id = rand::thread_rng().gen::<[u8; 32]>().to_vec();
        let email = body.email.clone();
        let created_at = Utc::now();
        let expires_at = created_at + chrono::Duration::hours(12);
        let token = TBEmailVerificationToken {
            id,
            email,
            expires_at,
            created_at,
        };

        let _existing = match TBEmailVerificationToken::find_by_mail(&body.email) {
            Ok(o) => {
                log::debug!("delete token");
                TBEmailVerificationToken::delete(&o.id)?;
                true
            }
            Err(_) => {
                log::debug!("no token existing");
                false
            }
        };

        let token = diesel::insert_into(email_verification_token::table)
            .values(&token)
            .get_result(&mut establish_connection()?)
            .unwrap();
        Ok(token)
    }

    pub fn delete(id: &Vec<u8>) -> Result<usize, SystemDBError> {
        let res = diesel::delete(
            email_verification_token::table.filter(email_verification_token::id.eq(id)),
        )
        .execute(&mut establish_connection()?)?;

        Ok(res)
    }
}

impl TBCaPayment {
    pub fn find(id: &i64) -> Result<Self, SystemDBError> {
        let cap = ca_payment::table
            .filter(ca_payment::id.eq(id))
            .first(&mut establish_connection()?)?;
        Ok(cap)
    }

    pub fn find_all(user_id_in: &i64) -> Result<Vec<Self>, SystemDBError> {
        let cap = ca_payment::table
            .filter(ca_payment::user_id.eq(user_id_in))
            .load::<TBCaPayment>(&mut establish_connection()?)?;
        Ok(cap)
    }

    pub fn find_user_contract(
        user_id_in: &i64,
        contract_id_in: &i64,
    ) -> Result<Vec<Self>, SystemDBError> {
        let cap = ca_payment::table
            .filter(ca_payment::user_id.eq(user_id_in))
            .filter(ca_payment::contract_id.eq(contract_id_in))
            .load::<TBCaPayment>(&mut establish_connection()?)?;
        Ok(cap)
    }

    pub fn find_user_st_open(user_id_in: &i64) -> Result<Vec<Self>, SystemDBError> {
        let cap = ca_payment::table
            .filter(ca_payment::user_id.eq(user_id_in))
            .filter(ca_payment::stauts_pa.eq("open"))
            .load::<TBCaPayment>(&mut establish_connection()?)?;
        Ok(cap)
    }

    pub async fn hash(&self) -> Result<String, SystemDBError> {
        TBCaPaymentHash::hash(self).await
    }

    pub async fn create(
        user_id: &i64,
        contract_id: &i64,
        value: &CaValue,
    ) -> Result<Self, SystemDBError> {
        let value = &serde_json::to_string(value)?;
        let new_pa = TBCaPaymentNew {
            user_id,
            contract_id,
            value,
            tx_hash: None,
            user_appr: None,
            drasil_appr: None,
            stauts_bl: None,
            stauts_pa: "new",
        };

        // ToDo: Check that payout sum cannot spent liquidity

        let pa = diesel::insert_into(ca_payment::table)
            .values(&new_pa)
            .get_result(&mut establish_connection()?)
            .unwrap();

        TBCaPaymentHash::create(&pa).await?;

        Ok(pa)
    }

    pub async fn approve_user(&self, user_signature: &str) -> Result<Self, SystemDBError> {
        TBCaPaymentHash::check(self).await?;
        let user_approval = diesel::update(ca_payment::table.find(&self.id))
            .set((
                ca_payment::user_appr.eq(Some(user_signature)),
                ca_payment::stauts_pa.eq("user approved"),
            ))
            .get_result::<TBCaPayment>(&mut establish_connection()?)?;
        TBCaPaymentHash::create(&user_approval).await?;
        Ok(user_approval)
    }

    pub async fn approve_drasil(&self, drsl_signature: &str) -> Result<Self, SystemDBError> {
        TBCaPaymentHash::check(self).await?;
        let drasil_approval = diesel::update(ca_payment::table.find(&self.id))
            .set((
                ca_payment::drasil_appr.eq(Some(drsl_signature)),
                ca_payment::stauts_pa.eq("fully approved"),
            ))
            .get_result::<TBCaPayment>(&mut establish_connection()?)?;
        TBCaPaymentHash::create(&drasil_approval).await?;
        Ok(drasil_approval)
    }

    pub fn cancel(&self) -> Result<Self, SystemDBError> {
        let cancel = diesel::update(ca_payment::table.find(&self.id))
            .set((
                ca_payment::stauts_pa.eq("canceled"),
                ca_payment::drasil_appr.eq::<Option<String>>(None),
                ca_payment::user_appr.eq::<Option<String>>(None),
            ))
            .get_result::<TBCaPayment>(&mut establish_connection()?)?;
        Ok(cancel)
    }

    // ToDO: build and submit payout transaction
    pub async fn execute(&self, pw: &str) -> Result<Self, SystemDBError> {
        TBCaPaymentHash::check(self).await?;
        let mut conn = establish_connection()?;
        let user = TBDrasilUser::get_user_by_user_id(&mut establish_connection()?, &self.user_id)?;
        let msg = TBCaPaymentHash::find_by_payid(&self.id)?[0]
            .payment_hash
            .clone();
        TBDrasilUser::verify_pw_userid(&self.user_id, &pw.to_string())?;
        match (
            user.verify_approval(&msg, &self.user_appr.clone().expect("Error: POT1001"))?,
            crate::admin::verify_approval_drsl(
                &msg,
                &self.drasil_appr.clone().expect("Error: POT1002"),
            )
            .await?,
        ) {
            (true, true) => (),
            _ => return Err(SystemDBError::Custom("Error: POT1003".to_string())),
        }

        //Trigger Build and submit payout transaction
        let contract = TBContracts::get_contract_uid_cid(self.user_id, self.contract_id)?;

        let mut gtxd = TxData::new(
            Some(self.contract_id as u64),
            vec![murin::wallet::b_decode_addr(&get_vaddr(&self.user_id).await?).await?],
            None,
            TransactionUnspentOutputs::new(),
            get_network_from_address(&contract.address)?,
            0,
        )?;

        let verified_addr = gtxd.get_senders_address(None).unwrap();

        let cv = serde_json::from_str::<CaValue>(&self.value)?.into_cvalue()?;
        let txo_values = vec![(&verified_addr, &cv, None)];

        // ToDo: Check that payout sum cannot spent liquidity

        let mut dbsync = match mimir::establish_connection() {
            Ok(conn) => conn,
            Err(e) => {
                return Err(SystemDBError::Custom(format!(
                    "ERROR could not connect to dbsync: '{:?}'",
                    e.to_string()
                )))
            }
        };
        let slot = match mimir::get_slot(&mut dbsync) {
            Ok(s) => s,
            Err(e) => {
                return Err(SystemDBError::Custom(format!(
                    "ERROR could not determine current slot: '{:?}'",
                    e.to_string()
                )))
            }
        };
        gtxd.set_current_slot(slot as u64);
        log::info!("DB Sync Slot: {}", slot);
        //ToDO:
        // - Find a solution for protocal parameters (maybe to database?) at the moment they are hardcoded in list / build_rwd
        let utxos = mimir::get_address_utxos(&mut dbsync, &contract.address)
            .expect("MimirError: cannot find address utxos");
        gtxd.set_inputs(utxos);

        log::debug!("Try to establish database connection...");
        let mut drasildbcon = crate::database::drasildb::establish_connection()?;

        log::debug!("Try to determine additional data...");
        let keyloc = crate::drasildb::TBMultiSigLoc::get_multisig_keyloc(
            &mut drasildbcon,
            &contract.contract_id,
            &contract.user_id,
            &contract.version,
        )?;

        let ident = crate::encryption::mident(
            &contract.user_id,
            &contract.contract_id,
            &contract.version,
            &contract.address,
        );
        let pkvs = crate::encryption::decrypt_pkvs(keyloc.pvks, &ident).await?;

        log::debug!("Try to build transaction...");
        let bld_tx =
            murin::stdtx::build_cpo::build_payment_tx(&gtxd, &pkvs, &txo_values, &contract.plutus)
                .await?;
        log::debug!("Try to create raw tx...");
        let tx = murin::utxomngr::RawTx::new(
            &bld_tx.get_tx_body(),
            &bld_tx.get_txwitness(),
            &bld_tx.get_tx_unsigned(),
            &bld_tx.get_metadata(),
            &gtxd.to_string(),
            &"CPayout".to_string(),
            &bld_tx.get_used_utxos(),
            &"".to_string(),
            &user.user_id,
            &contract.contract_id,
            &contract.version,
        );
        debug!("RAWTX data: {:?}", tx);

        let used_utxos = tx.get_usedutxos().clone();
        let txh = murin::finalize_rwd(
            &hex::encode(&murin::clib::TransactionWitnessSet::new().to_bytes()),
            tx,
            pkvs,
        )
        .await?;
        murin::utxomngr::usedutxos::store_used_utxos(
            &txh,
            &murin::TransactionUnspentOutputs::from_hex(&used_utxos)?,
        )?;

        // On Success update status
        let exec = diesel::update(ca_payment::table.find(&self.id))
            .set((
                ca_payment::stauts_pa.eq("transfer in process"),
                ca_payment::stauts_bl.eq("transaction submit"),
                ca_payment::tx_hash.eq(Some(txh)),
            ))
            .get_result::<TBCaPayment>(&mut conn)?;
        TBCaPaymentHash::create(&exec).await?;
        Ok(exec)
    }

    // ToDo: Triggered by Monitoring Tool
    pub async fn st_confirmed(&self) -> Result<Self, SystemDBError> {
        TBCaPaymentHash::check(self).await?;
        let confi = diesel::update(ca_payment::table.find(&self.id))
            .set((
                ca_payment::stauts_bl.eq("confirmed"),
                ca_payment::stauts_bl.eq("transaction on chain"),
            ))
            .get_result::<TBCaPayment>(&mut establish_connection()?)?;
        Ok(confi)
    }
}

impl TBCaPaymentHash {
    pub fn find(&self) -> Result<Self, SystemDBError> {
        let caph = ca_payment_hash::table
            .filter(ca_payment_hash::id.eq(&self.id))
            .first(&mut establish_connection()?)?;
        Ok(caph)
    }

    pub fn find_by_payid(payment_id_in: &i64) -> Result<Vec<Self>, SystemDBError> {
        let caph = ca_payment_hash::table
            .filter(ca_payment_hash::payment_id.eq(payment_id_in))
            .order_by(ca_payment_hash::created_at.desc())
            .load::<TBCaPaymentHash>(&mut establish_connection()?)?;
        Ok(caph)
    }

    pub async fn hash(tbcapay: &TBCaPayment) -> Result<String, SystemDBError> {
        let payout_addr =
            TBDrasilUser::get_user_by_user_id(&mut establish_connection()?, &tbcapay.user_id)?
                .cardano_wallet
                .unwrap();

        let vaddr = get_vaddr(&tbcapay.user_id).await?;

        if payout_addr != vaddr {
            return Err(SystemDBError::Custom(
                "Exxxx: verified addresse discrepancy".to_string(),
            ));
        }
        let last_hash = match TBCaPaymentHash::find_by_payid(&tbcapay.id) {
            Ok(o) => Some(o[0].payment_hash.clone()),
            Err(e) => {
                if e.to_string() == "NotFound" {
                    None
                } else {
                    return Err(SystemDBError::Custom(e.to_string()));
                }
            }
        };

        let mut hasher = sha2::Sha224::new();
        if let Some(hash) = last_hash {
            hasher.update((hash).as_bytes());
        }
        hasher.update(payout_addr.as_bytes());
        hasher.update((tbcapay.id).to_ne_bytes());
        hasher.update((tbcapay.user_id).to_ne_bytes());
        hasher.update((tbcapay.contract_id).to_ne_bytes());
        hasher.update((tbcapay.value).as_bytes());
        if let Some(tx_hash) = tbcapay.tx_hash.as_ref() {
            hasher.update(tx_hash.as_bytes());
        }
        if let Some(user_appr) = tbcapay.user_appr.as_ref() {
            hasher.update(user_appr.as_bytes());
        }
        if let Some(drasil_appr) = tbcapay.drasil_appr.as_ref() {
            hasher.update(drasil_appr.as_bytes());
        }
        hasher.update((tbcapay.created_at).to_string().as_bytes());
        hasher.update((tbcapay.updated_at).to_string().as_bytes());
        Ok(hex::encode(hasher.finalize()))
    }

    pub async fn create(tbcapay: &TBCaPayment) -> Result<Self, SystemDBError> {
        let hash = TBCaPaymentHash::hash(tbcapay).await?;
        let new_pah = TBCaPaymentHashNew {
            payment_id: &tbcapay.id,
            payment_hash: &hash,
        };

        let pah = diesel::insert_into(ca_payment_hash::table)
            .values(&new_pah)
            .get_result(&mut establish_connection()?)
            .unwrap();
        Ok(pah)
    }

    pub async fn check(tbcapay: &TBCaPayment) -> Result<(), SystemDBError> {
        let last_hash = &TBCaPaymentHash::find_by_payid(&tbcapay.id)?[0].payment_hash;
        let hash = TBCaPaymentHash::hash(tbcapay).await?;
        if hash != *last_hash {
            return Err(SystemDBError::Custom(
                "PayoutHash changed, check failed!".to_string(),
            ));
        };
        Ok(())
    }
}
