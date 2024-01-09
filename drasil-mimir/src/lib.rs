#[macro_use]
extern crate diesel;
pub mod schema;
use schema::*;

pub mod models;
pub use models::*;
pub(crate) mod error;
pub use error::MimirError;

pub mod api;
pub use api::*;

use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenv::dotenv;
use std::env;

extern crate dotenv;
extern crate pretty_env_logger;

pub fn establish_connection() -> Result<PgConnection, error::MimirError> {
    dotenv().ok();

    let database_url = env::var("DBSYNC_DB_URL")?;
    Ok(PgConnection::establish(&database_url)?)
}
