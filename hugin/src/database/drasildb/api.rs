/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use super::*;
use crate::schema::{contracts, email_verification_token, multisig_keyloc};
use diesel::pg::upsert::on_constraint;
use murin::MurinError;

impl TBContracts {
    pub fn get_drasil_liquidity_wallet(conn: &PgConnection) -> Result<TBContracts, MurinError> {
        use crate::schema::contracts::dsl::*;
        let result = contracts
            .filter(
                contract_type
                    .eq(&crate::datamodel::hephadata::ContractType::DrasilAPILiquidity.to_string()),
            )
            .first::<TBContracts>(&*conn)?;
        Ok(result)
    }

    pub fn get_contract_for_user(
        conn: &PgConnection,
        uid: i64,
        ctype: String,
        vers: Option<f32>,
    ) -> Result<TBContracts, MurinError> {
        use crate::schema::contracts::dsl::*;
        let result = contracts
            .filter(user_id.eq(&uid))
            .filter(contract_type.eq(&ctype))
            .order(version.desc())
            .load::<TBContracts>(&*conn)?;

        let err = MurinError::new(&format!(
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
        conn: &PgConnection,
        uid: i64,
        ctype: String,
        vers: Option<f32>,
    ) -> Result<TBContracts, MurinError> {
        use crate::schema::contracts::dsl::*;
        let result = contracts
            .filter(user_id.eq(&uid))
            .filter(contract_type.eq(&ctype))
            .filter(depricated.eq(false))
            .order(version.desc())
            .load::<TBContracts>(&*conn)?;

        let err = MurinError::new(&format!(
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
    ) -> Result<Vec<TBContracts>, MurinError> {
        use crate::schema::contracts::dsl::*;

        let conn = establish_connection()?;

        let result = contracts
            .filter(user_id.eq(&(uid)))
            .filter(contract_type.eq(&ctype))
            .order(contract_id.asc())
            .load::<TBContracts>(&conn)?;

        Ok(result)
    }

    pub fn get_contract_uid_cid(
        user_id_in: i64,
        contract_id_in: i64,
    ) -> Result<TBContracts, MurinError> {
        log::debug!("try to get data from contracts table: ");
        let result = contracts::table
            .filter(contracts::user_id.eq(&user_id_in))
            .filter(contracts::contract_id.eq(&contract_id_in))
            .load::<TBContracts>(&establish_connection()?);

        log::debug!("Result: {:?}", result);

        Ok(result?[0].clone())
    }

    pub fn get_next_contract_id(user_id_in: i64) -> Result<i64, MurinError> {
        use crate::schema::contracts::dsl::*;
        let result = contracts
            .filter(user_id.eq(user_id_in))
            .select(contract_id)
            .order(contract_id.desc())
            .limit(1)
            .load::<i64>(&establish_connection()?)?;

        let mut contract_id_new = 0;
        if !result.is_empty() {
            contract_id_new = result[0] + 1;
        }
        Ok(contract_id_new)
    }

    pub fn get_contract_by_id(conn: &PgConnection, id_in: i64) -> Result<TBContracts, MurinError> {
        use crate::schema::contracts::dsl::*;
        let result = contracts.find(id_in).first::<TBContracts>(&*conn)?;
        Ok(result)
    }

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
    ) -> Result<TBContracts, MurinError> {
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
            .get_result::<TBContracts>(&establish_connection()?)?)
    }

    pub fn update_contract<'a>(
        conn: &PgConnection,
        id_in: &'a i64,
        contract_id_new: &'a i64,
        description_new: Option<&'a str>,
        depricated_new: &'a bool,
    ) -> Result<TBContracts, MurinError> {
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
        conn: &PgConnection,
        user_id_in: &'a i64,
        contract_id_in: &'a i64,
        depricated_in: &'a bool,
    ) -> Result<TBContracts, MurinError> {
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
    pub async fn create_multisig_keyloc<'a>(
        user_id: &'a i64,
        contract_id: &'a i64,
        version: &'a f32,
        ca: &'a String,
        fee_wallet_addr: Option<&'a String>,
        fee: Option<&'a i64>,
        pvks: &'a Vec<String>,
        depricated: &'a bool,
    ) -> Result<TBMultiSigLoc, MurinError> {
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
            .get_result::<TBMultiSigLoc>(&establish_connection()?)?)
    }

    pub fn get_multisig_keyloc(
        conn: &PgConnection,
        contract_id_in: &i64,
        user_id_in: &i64,
        version_in: &f32,
    ) -> Result<TBMultiSigLoc, MurinError> {
        use crate::schema::multisig_keyloc::dsl::*;
        let result = multisig_keyloc
            .filter(contract_id.eq(&contract_id_in))
            .filter(user_id.eq(&user_id_in))
            .filter(version.eq(&version_in))
            .load::<TBMultiSigLoc>(&*conn)?;

        let err = MurinError::new(&format!("no multisig key location found for contract-id: '{}' User-id: '{}'  , version: '{}'; \n Result: {:?}"
                ,contract_id_in, user_id_in, version_in, result));

        if let Some(r) = result.get(0) {
            return Ok(r.clone());
        };

        Err(err)
    }
}

impl TBDrasilUser {
    fn get_next_user_id(conn: &PgConnection) -> Result<i64, MurinError> {
        use crate::schema::drasil_user::dsl::*;
        let result = drasil_user
            .select(user_id)
            .order_by(user_id.desc())
            .first::<i64>(&*conn)?;
        Ok(result + 1)
    }

    fn get_user_by_mail(
        conn: &PgConnection,
        email_in: &String,
    ) -> Result<TBDrasilUser, MurinError> {
        use crate::schema::drasil_user::dsl::*;
        let result = drasil_user
            .filter(email.eq(email_in))
            .first::<TBDrasilUser>(&*conn)?;
        Ok(result)
    }

    pub fn get_user_by_user_id(
        conn: &PgConnection,
        user_id_in: &i64,
    ) -> Result<TBDrasilUser, MurinError> {
        use crate::schema::drasil_user::dsl::*;
        let result = drasil_user
            .filter(user_id.eq(user_id_in))
            .first::<TBDrasilUser>(&*conn)?;
        Ok(result)
    }

    pub fn verify_pw_user(email: &String, pwd: &String) -> Result<TBDrasilUser, MurinError> {
        let conn = establish_connection()?;
        use argon2::{
            password_hash::{PasswordHash, PasswordVerifier},
            Argon2,
        };
        let user = TBDrasilUser::get_user_by_mail(&conn, email)?;
        Argon2::default().verify_password(pwd.as_bytes(), &PasswordHash::new(&user.pwd)?)?;
        Ok(user)
    }

    pub fn create_user<'a>(
        conn: &PgConnection,
        api_pubkey: Option<&'a String>,
        uname: &'a String,
        email: &'a String,
        pwd: &'a String, // needs to be hashed already at this stage
        role: &'a String,
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
    ) -> Result<TBDrasilUser, MurinError> {
        log::debug!("create user");
        use argon2::{
            password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
            Argon2,
        };
        let nuser_id = TBDrasilUser::get_next_user_id(conn);
        let user_id = match nuser_id {
            Ok(id) => id,
            Err(e) => {
                if e.to_string() == *"Not User ID Found" {
                    0
                } else {
                    return Err(e);
                }
            }
        };
        log::debug!("u1");
        let password_hash = Argon2::default()
            .hash_password(pwd.as_bytes(), &SaltString::generate(&mut OsRng))?
            .to_string();

        let new_user = TBDrasilUserNew {
            user_id: &user_id,
            api_pubkey,
            uname,
            email,
            pwd: &password_hash,
            role,
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
        };
        log::debug!("u2");
        let user = match TBDrasilUser::get_user_by_mail(conn, email) {
            Ok(u) => {
                if u.email_verified {
                    return Err(MurinError::new("User exists and is verified already"));
                }
                u
            }
            Err(e) => diesel::insert_into(drasil_user::table)
                .values(&new_user)
                .on_conflict(on_constraint("unique_email"))
                .do_nothing()
                .get_result::<TBDrasilUser>(conn)?,
        };
        log::debug!("u3");
        Ok(user)
    }

    pub fn verify_email(email_in: &String) -> Result<TBDrasilUser, MurinError> {
        use crate::schema::drasil_user::dsl::*;
        let conn = establish_connection()?;
        let user = TBDrasilUser::get_user_by_mail(&conn, email_in)?;

        let user_updated = diesel::update(drasil_user.find(user.id))
            .set((email_verified.eq(true),))
            .get_result::<TBDrasilUser>(&conn)?;

        Ok(user_updated)
    }

    pub fn update_api_key<'a>(
        user_id_in: &'a i64,
        token: &'a String,
    ) -> Result<TBDrasilUser, MurinError> {
        use crate::schema::drasil_user::dsl::*;
        let conn = establish_connection()?;

        let user_updated = diesel::update(drasil_user.find(user_id_in))
            .set((api_pubkey.eq(Some(token)),))
            .get_result::<TBDrasilUser>(&conn)?;

        Ok(user_updated)
    }
}

impl TBEmailVerificationToken {
    pub fn find(id: &Vec<u8>) -> Result<Self, MurinError> {
        let conn = establish_connection()?;

        let token = email_verification_token::table
            .filter(email_verification_token::id.eq(id))
            .first(&conn)?;

        Ok(token)
    }

    pub fn find_by_mail(email_in: &str) -> Result<Self, MurinError> {
        let conn = establish_connection()?;

        let token = email_verification_token::table
            .filter(email_verification_token::email.eq(email_in))
            .first(&conn)?;

        Ok(token)
    }

    pub fn create(body: TBEmailVerificationTokenMessage) -> Result<Self, MurinError> {
        use rand::Rng;

        let conn = establish_connection()?;

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

        let existing = match TBEmailVerificationToken::find_by_mail(&body.email) {
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
        /*
        .on_conflict(email_verification_token::email)
                   .do_update()
                   .set((
                       email_verification_token::id.eq(&token.id),
                       email_verification_token::created_at.eq(&token.created_at),
                       email_verification_token::expires_at.eq(&token.expires_at),
                   )) */
        let token = diesel::insert_into(email_verification_token::table)
            .values(&token)
            .get_result(&conn)
            .unwrap();
        Ok(token)
    }

    pub fn delete(id: &Vec<u8>) -> Result<usize, MurinError> {
        let conn = establish_connection()?;

        let res = diesel::delete(
            email_verification_token::table.filter(email_verification_token::id.eq(id)),
        )
        .execute(&conn)?;

        Ok(res)
    }
}

impl TBMultiSigs {}
