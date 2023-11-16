use diesel::pg::upsert::on_constraint;
use drasil_dvltath::vault::kv::{vault_get, vault_store};
use drasil_murin::crypto::{Ed25519Signature, PrivateKey, PublicKey};
use drasil_murin::wallet;
use error::SystemDBError;
use sha2::Digest;

use super::*;
use crate::admin::get_vaddr;
use crate::client::connect;
use crate::encryption::{decrypt, encrypt};
use crate::schema::{contracts, email_verification_token, multisig_keyloc};
use crate::{BuildMultiSig, Operation, TransactionPattern};

impl TBContracts {
    pub fn get_all_active_rwd_contracts() -> Result<Vec<TBContracts>, SystemDBError> {
        use crate::schema::contracts::dsl::*;
        let result = contracts
            .filter(
                contract_type
                    .eq(&crate::datamodel::models::MultiSigType::SpoRewardClaim.to_string()),
            )
            .filter(depricated.eq(false))
            .load::<TBContracts>(&mut establish_connection()?)?;
        Ok(result)
    }

    pub fn get_liquidity_wallet(user_id_in: &i64) -> Result<TBContracts, SystemDBError> {
        use crate::schema::contracts::dsl::*;
        let result = contracts
            .filter(
                contract_type
                    .eq(&crate::datamodel::models::ContractType::DrasilAPILiquidity.to_string()),
            )
            .filter(user_id.eq(user_id_in))
            .first::<TBContracts>(&mut establish_connection()?)?;
        Ok(result)
    }

    pub fn get_contract_liquidity(&self) -> drasil_murin::utils::BigNum {
        let e = self.external_lqdty.unwrap_or(0);
        let d = self.drasil_lqdty.unwrap_or(0);
        let o = self.customer_lqdty.unwrap_or(0);
        drasil_murin::utils::to_bignum((e + d + o) as u64)
    }

    pub fn get_contract_for_user(
        uid: i64,
        ctype: String,
        vers: Option<f32>,
    ) -> Result<TBContracts, SystemDBError> {
        use crate::schema::contracts::dsl::*;
        let result = contracts
            .filter(user_id.eq(&uid))
            .filter(contract_type.eq(&ctype))
            .order(version.desc())
            .load::<TBContracts>(&mut establish_connection()?)?;

        let err = SystemDBError::Custom(format!(
            "no contract found for user-id: '{uid}' and contract type '{ctype}'"
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
            .load::<TBContracts>(&mut establish_connection()?)?;

        let err = SystemDBError::Custom(format!(
            "no contract found for user-id: '{uid}' and contract type '{ctype}'"
        ));

        if let Some(v) = vers {
            let result: Vec<TBContracts> = result
                .into_iter()
                .filter(|elem| elem.version == v)
                .collect();
            if let Some(r) = result.get(0) {
                return Ok(r.clone());
            };
        } else if let Some(r) = result.get(0) {
            return Ok(r.clone());
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
        log::debug!("input data: u:{},c:{} ", user_id_in, contract_id_in);
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

    pub fn get_contract_by_id(id_in: i64) -> Result<TBContracts, SystemDBError> {
        use crate::schema::contracts::dsl::*;
        let result = contracts
            .find(id_in)
            .first::<TBContracts>(&mut establish_connection()?)?;
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
            .get_result::<TBContracts>(&mut establish_connection()?)?;

        Ok(contract)
    }

    pub fn depricate_contract<'a>(
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
        .get_result::<TBContracts>(&mut establish_connection()?)?;
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
        contract_id_in: &i64,
        user_id_in: &i64,
        version_in: &f32,
    ) -> Result<TBMultiSigLoc, SystemDBError> {
        use crate::schema::multisig_keyloc::dsl::*;
        let result = multisig_keyloc
            .filter(contract_id.eq(&contract_id_in))
            .filter(user_id.eq(&user_id_in))
            .filter(version.eq(&version_in))
            .load::<TBMultiSigLoc>(&mut establish_connection()?)?;

        let err = SystemDBError::Custom(format!("no multisig key location found for contract-id: '{contract_id_in}' User-id: '{user_id_in}'  , version: '{version_in}'; \n Result: {result:?}"
                ));

        if let Some(r) = result.get(0) {
            return Ok(r.clone());
        };

        Err(err)
    }
}

impl TBDrasilUser {
    fn get_next_user_id() -> Result<i64, SystemDBError> {
        use crate::schema::drasil_user::dsl::*;
        let result = drasil_user
            .select(user_id)
            .order_by(user_id.desc())
            .first::<i64>(&mut establish_connection()?)?;
        Ok(result + 1)
    }

    fn get_user_by_mail(email_in: &String) -> Result<TBDrasilUser, SystemDBError> {
        use crate::schema::drasil_user::dsl::*;
        let result = drasil_user
            .filter(email.eq(email_in))
            .first::<TBDrasilUser>(&mut establish_connection()?)?;
        Ok(result)
    }

    pub fn get_user_by_user_id(user_id_in: &i64) -> Result<TBDrasilUser, SystemDBError> {
        use crate::schema::drasil_user::dsl::*;
        let result = drasil_user
            .filter(user_id.eq(user_id_in))
            .first::<TBDrasilUser>(&mut establish_connection()?)?;
        Ok(result)
    }

    pub fn verify_pw_user(email: &String, pwd: &String) -> Result<TBDrasilUser, SystemDBError> {
        use argon2::{
            password_hash::{PasswordHash, PasswordVerifier},
            Argon2,
        };
        let user = TBDrasilUser::get_user_by_mail(email);
        info!("user: {:?}", user);
        let user = user?;
        Argon2::default().verify_password(pwd.as_bytes(), &PasswordHash::new(&user.pwd).unwrap()).unwrap();
        Ok(user)
    }

    pub fn verify_pw_userid(user_id: &i64, pwd: &String) -> Result<TBDrasilUser, SystemDBError> {
        use argon2::{
            password_hash::{PasswordHash, PasswordVerifier},
            Argon2,
        };
        let user = TBDrasilUser::get_user_by_user_id(user_id).unwrap();
        let pw = Argon2::default().verify_password(pwd.as_bytes(), &PasswordHash::new(&user.pwd)?);
        info!("pw: {:?}", pw);
        let pw = pw?;
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
        let nuser_id = TBDrasilUser::get_next_user_id();
        let user_id = match nuser_id {
            Ok(id) => id,
            Err(e) => {
                log::error!("Error did not found any userid: {:?}", e.to_string());
                if e.to_string() == *"Record not found" {
                    0
                } else {
                    return Err(e);
                }
            }
        };
        let password_hash = Argon2::default()
            .hash_password(pwd.as_bytes(), &SaltString::generate(&mut OsRng))?
            .to_string();

        let (privkey, pubkey, pubkeyhash) = wallet::create_drslkeypair();
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
        let user = match TBDrasilUser::get_user_by_mail(email) {
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
        let user = TBDrasilUser::get_user_by_mail(email_in)?;

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
        let pk_s = if let Some(pk_t) = self.drslpubkey.as_ref() {
            pk_t
        }else{
            return Err(SystemDBError::Custom("No public key defined".to_string()));
        };
        let pk = PublicKey::from_bech32(&pk_s)?;
        
        let pkh = hex::encode(PublicKey::from_bech32(&pk_s)?.hash().to_bytes());
        let privkey = vault_get(&pkh).await;
        let privkey = PrivateKey::from_bech32(&decrypt(privkey.get("prvkey").unwrap(), pw)?)?;

        let sign = privkey.sign(msg.as_bytes());
        let verify = pk.verify(msg.as_bytes(), &sign);
        assert!(verify);
        Ok(sign.to_hex())
    }

    pub fn verify_approval(&self, msg: &str, sign: &str) -> Result<bool, SystemDBError> {
        let pk_s = if let Some(pk_t) = self.drslpubkey.as_ref() {
            pk_t
        }else{
            return Err(SystemDBError::Custom("No public key defined".to_string()));
        };
        let pk = PublicKey::from_bech32(&pk_s)?;
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
            .filter(ca_payment::status_pa.eq("open"))
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
        pw: &String,
    ) -> Result<Self, SystemDBError> {
        log::debug!("Verify password...");
        TBDrasilUser::verify_pw_userid(user_id, &pw.to_string())?;
        let value = &serde_json::to_string(value)?;
        let new_pa = TBCaPaymentNew {
            user_id,
            contract_id,
            value,
            tx_hash: None,
            user_appr: None,
            drasil_appr: None,
            status_bl: None,
            status_pa: "new",
        };

        let pa = diesel::insert_into(ca_payment::table)
            .values(&new_pa)
            .get_result(&mut establish_connection()?);

        log::debug!("created?: {:?}", pa);
        let pa = pa?;
        TBCaPaymentHash::create(&pa).await?;

        Ok(pa)
    }

    pub async fn approve_user(&self, user_signature: &str) -> Result<Self, SystemDBError> {
        TBCaPaymentHash::check(self).await?;
        let user_approval = diesel::update(ca_payment::table.find(&self.id))
            .set((
                ca_payment::user_appr.eq(Some(user_signature)),
                ca_payment::status_pa.eq("user approved"),
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
                ca_payment::status_pa.eq("fully approved"),
            ))
            .get_result::<TBCaPayment>(&mut establish_connection()?)?;
        TBCaPaymentHash::create(&drasil_approval).await?;
        Ok(drasil_approval)
    }

    pub fn cancel(&self) -> Result<Self, SystemDBError> {
        let cancel = diesel::update(ca_payment::table.find(&self.id))
            .set((
                ca_payment::status_pa.eq("canceled"),
                ca_payment::drasil_appr.eq::<Option<String>>(None),
                ca_payment::user_appr.eq::<Option<String>>(None),
            ))
            .get_result::<TBCaPayment>(&mut establish_connection()?)?;
        Ok(cancel)
    }

    // ToDO: build and submit payout transaction
    pub async fn execute(&self, pw: &str) -> Result<Self, SystemDBError> {
        //TBCaPaymentHash::check(self).await?;
        //let msg = TBCaPaymentHash::find_by_payid(&self.id)?[0]
        //    .payment_hash
        //    .clone();
        log::debug!("Verify password...");
        TBDrasilUser::verify_pw_userid(&self.user_id, &pw.to_string())?;
        /*
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
        */
        log::debug!("connect to odin...");
        let mut client = connect(std::env::var("ODIN_URL").unwrap()).await.unwrap();
        let cmd = BuildMultiSig::new(
            self.user_id.try_into().unwrap(),
            crate::MultiSigType::CustomerPayout,
            TransactionPattern::new_empty(
                self.user_id.try_into().unwrap(),
                &Operation::CPO {
                    po_id: self.id,
                    pw: pw.to_string(),
                },
                0b0,
            ),
        );
        log::info!("try to build payout in odin ...");
        let result = match client.build_cmd::<BuildMultiSig>(cmd).await {
            Ok(s) => serde_json::from_str::<TBCaPayment>(&s)?,
            Err(_) => {
                return Err(SystemDBError::Custom(
                    "Error: odin could not finialize payout transaction".to_string(),
                ));
            }
        };

        Ok(result)
    }

    // ToDo: Triggered by Monitoring Tool
    pub async fn st_confirmed(&self) -> Result<Self, SystemDBError> {
        TBCaPaymentHash::check(self).await?;
        let confi = diesel::update(ca_payment::table.find(&self.id))
            .set((
                ca_payment::status_bl.eq("confirmed"),
                ca_payment::status_bl.eq("transaction on chain"),
            ))
            .get_result::<TBCaPayment>(&mut establish_connection()?)?;
        Ok(confi)
    }

    pub async fn update_txhash(&self, txh: &String) -> Result<TBCaPayment, SystemDBError> {
        Ok(diesel::update(ca_payment::table.find(&self.id))
            .set((
                ca_payment::status_pa.eq("transfer in process"),
                ca_payment::status_bl.eq("transaction submit"),
                ca_payment::tx_hash.eq(Some(txh)),
            ))
            .get_result::<TBCaPayment>(&mut establish_connection()?)?)
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
        log::debug!("Try to create payout hash...");

        let payout_addr = get_vaddr(&tbcapay.user_id).await?;
        //ToDo: Store hash in user table of verified address and check against that
        /*
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
        */
        let last_hash: Option<String> = None;
        log::debug!("Try to run hasher...");
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
        log::debug!("PO hash: {:?}", hash);
        let new_pah = TBCaPaymentHashNew {
            payment_id: &tbcapay.id,
            payment_hash: &hash,
        };

        let pah = diesel::insert_into(ca_payment_hash::table)
            .values(&new_pah)
            .get_result(&mut establish_connection()?);
        log::debug!("created hash?: {:?}", pah);
        let pah = pah?;

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
