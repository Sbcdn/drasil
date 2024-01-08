pub mod models;
pub mod reward_handling;
pub mod stake;
pub mod whitelists;

use csv::WriterBuilder;
use drasil_murin::MurinError;

use reward_handling::{check_contract_is_active, get_token_whitelist, handle_lists};

use crate::handlers::reward_calculation::models::RewardTable;

pub async fn reward_calculation(epoch: Option<i64>, from: Option<bool>) -> Result<(), MurinError> {
    pretty_env_logger::init();

    let current_epoch = drasil_mimir::get_epoch(
        &mut drasil_mimir::establish_connection().map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())? as i64;
    let calc_epoch = current_epoch - 2;
    log::debug!("Current Epoch: {}", current_epoch);
    log::debug!("Calculation Epoch: {}", calc_epoch);
    if epoch.is_some() && epoch.unwrap() > calc_epoch {
        return Err(MurinError::new(
            "It is not possible to calculate rewards for the current or future epochs",
        ));
    }

    let mut i = calc_epoch;
    if epoch.is_some() {
        i = epoch.unwrap();
    };
    let mut table = Vec::<RewardTable>::new();
    if let Some(b) = from {
        while i < calc_epoch && b {
            let mut whitelist = get_token_whitelist(calc_epoch).await?;
            whitelist.retain(|w| w.start_epoch <= i);
            log::debug!("Whitelist: {:?}", whitelist);
            for mut entry in whitelist {
                if check_contract_is_active(&entry)? {
                    log::debug!("Entered: {:?}", entry);
                    handle_lists(&mut entry, i, &mut table).await?
                    //   tokio::spawn(async move {
                    //       if let Err(err) = handle_pools(&mut entry,i).await {
                    //error!(cause = ?err, "calculation error for whitelist entry");
                    //           panic!("calculation error for whitelist entry: {:?}",err);
                    //       }
                    //   });
                }
            }
            i += 1;
        }
        log::debug!(
            "Rewards successfully calucalted for epochs {:?} to {:?}",
            epoch,
            i
        );
    } else {
        let mut whitelist = get_token_whitelist(calc_epoch).await?;
        whitelist.retain(|w| w.start_epoch <= i);
        log::debug!("Whitelist: {:?}", whitelist);
        for mut entry in whitelist {
            if check_contract_is_active(&entry)? {
                handle_lists(&mut entry, i, &mut table).await?
            }
        }
        log::debug!("Rewards successfully calucalted for epoch: {:?}", i);
    }

    let mut bpath = "/".to_string();

    bpath.push_str(&(calc_epoch.to_string() + "_"));
    bpath.push_str(&chrono::offset::Utc::now().to_string());
    bpath.push_str(".csv");
    let mut wtr2 = WriterBuilder::new().from_writer(vec![]);
    for entry in table {
        let mut e = entry.twldata.to_str_vec();
        e.extend(
            &mut vec![
                entry.calc_date.to_string(),
                entry.current_epoch.to_string(),
                entry.earned_epoch.to_string(),
                entry.total_earned_epoch.to_string(),
            ]
            .into_iter(),
        );

        wtr2.write_record(&e).map_err(|e| e.to_string())?;
    }
    // ToDo:
    // Store calculation Protocol into database
    let _data = String::from_utf8(wtr2.into_inner().map_err(|e| e.to_string())?)?;

    Ok(())
}
