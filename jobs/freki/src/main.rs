mod models;
mod rwd_handling;
mod stake;
mod whitelists;

use csv::WriterBuilder;
use models::*;
use rwd_handling::{check_contract_is_active, get_token_whitelist, handle_lists};
use structopt::StructOpt;

use s3::bucket::Bucket;
use s3::creds::Credentials;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "Reward Calculator",
    about = "Calculates rewards for the drasil - freeloaderz SmartClaimz system."
)]
struct Opt {
    #[structopt(short, long, about = "the epoch rewards should be calcualted for")]
    epoch: Option<i64>,

    #[structopt(
        short,
        long,
        about = "calc from the given epoch up to the latest possible one"
    )]
    from: Option<bool>,
}

#[tokio::main]
pub async fn main() -> Result<()> {
    let opt = Opt::from_args();
    pretty_env_logger::init();

    let current_epoch = mimir::get_epoch(&mut mimir::establish_connection()?)? as i64;
    let calc_epoch = current_epoch - 2;
    log::debug!("Current Epoch: {}", current_epoch);
    log::debug!("Calculation Epoch: {}", calc_epoch);
    if opt.epoch.is_some() && opt.epoch.unwrap() > calc_epoch {
        return Err(gungnir::RWDError::new(
            "It is not possible to calculate rewards for the current or future epochs",
        )
        .into());
    }

    let mut i = calc_epoch;
    if opt.epoch.is_some() {
        i = opt.epoch.unwrap();
    };
    let mut table = Vec::<RewardTable>::new();
    if let Some(b) = opt.from {
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
            opt.epoch,
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

        wtr2.write_record(&e)?;
    }
    let data = String::from_utf8(wtr2.into_inner()?)?;
    let bucket_name = "freki-protocols";
    let region = "us-east-2".parse().unwrap();
    let credentials = Credentials::default().unwrap();
    let bucket = Bucket::new(bucket_name, region, credentials)?;
    let response_data = bucket.put_object(bpath, data.as_bytes()).await?;
    log::debug!("S3 Response: {:?}", response_data.1);

    Ok(())
}
