pub(crate) mod reward_handler;
pub(crate) use reward_handler::handle_rewardclaim;

pub(crate) mod mcollection_handler;
pub(crate) use mcollection_handler::handle_collection_mint;

pub(crate) mod moneshot_handler;
pub(crate) use moneshot_handler::handle_onehshot_mint;

pub(crate) mod cpo_handler;
pub(crate) use cpo_handler::handle_customer_payout;

//pub(crate) mod testrwd_handler;
//pub(crate) use testrwd_handler::handle_testrewards;
