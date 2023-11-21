//! Application configuration data structure

use config::Config;
use secrecy::Secret;
use serde::Deserialize;
use serde_aux::field_attributes::deserialize_number_from_string;

use crate::error::Error;

#[derive(Clone, Debug, Deserialize)]
/// Settings is the application configuration type.
pub struct Settings {
    /// The application settings.
    pub application: AppSettings,

    /// JWT configurations
    pub jwt: JwtSettings,

    /// ODIN client settings
    pub odin: OdinSettings,
}

/// The application level configuration settings.
#[derive(Clone, Debug, Deserialize)]
pub struct AppSettings {
    /// Application port number.
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,

    /// The application IP address or hostname.
    pub host: String,
}

impl AppSettings {
    /// Returns the connection for the application.
    pub fn connection_string(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// The JWT configuration settings.
#[derive(Clone, Debug, Deserialize)]
pub struct JwtSettings {
    /// This is the secret for encoding and decoding token.
    pub(super) pub_key: Secret<String>,
}

/// Odin service client configuration data.
#[derive(Clone, Debug, Deserialize)]
pub struct OdinSettings {
    /// Connection url
    pub url: String,
}

impl Settings {
    /// Loads the configuration data.
    pub fn load() -> Result<Self, Error> {
        let settings_dir = std::env::current_dir()
            .map(|dir| dir.join("settings"))
            .expect("failed to resolve current directory");

        let network: Network = std::env::var("HEIMDALLR_NETWORK")
            .unwrap_or_else(|_| "local-dev-net".into())
            .try_into()
            .expect("failed to parse HEIMDALLR_NETWORK");
        // Initialize the configuration reader with the base configuration data.
        let builder = Config::builder()
            .add_source(config::File::from(settings_dir.join("base")).required(false))
            .add_source(config::File::from(settings_dir.join(network.as_str())).required(true))
            // We shouldn't use environment variables for configuration because they can be
            // accessed by another process.
            // with the below configuration, an environment variable settings like
            // HEIMDALLR_APPLICATION__PORT would set Settings.application.port
            .add_source(config::Environment::with_prefix("HEIMDALLR").separator("__"));

        let settings = builder.build().and_then(Config::try_deserialize)?;
        Ok(settings)
    }
}

/// This macro create a network environment type.
macro_rules! define_network_environment {
    (
        $network:ident;
        $($net_env:ident => $literal:literal,)*

    ) => {

        #[doc = concat!("The ", stringify!($network), " type is the network environment")]
        #[derive(Debug, Copy, Clone)]
        #[non_exhaustive]
        #[allow(missing_docs, clippy::missing_docs_in_private_items,)]
        pub enum $network {
            $($net_env,)*
        }

        impl $network {
            #[doc="The list of all supported environments"]
            const NETWORKS: &'static [&'static str] = &[$($literal,)*];


            #[doc=concat!("Return the string slice representation of ", stringify!($network), " variant")]
            const fn as_str(&self) -> &str {
                Self::NETWORKS[*self as usize]
            }
        }

        impl std::fmt::Display for $network {
            fn fmt(&self,  f: &mut std::fmt::Formatter<'_>)-> std::fmt::Result{
                write!(f, "{}", self.as_str())
            }
        }

        impl TryFrom<String> for $network {
            type Error = String;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                let net = match value.to_lowercase().as_str() {
                    $($literal => Self::$net_env,)*
                    _ => return Err(format!(
                        "`{value}` is not is supported network environment. Use {}",
                        Self::NETWORKS.join(", "))),
                };

                Ok(net)
            }
        }

    };
}

define_network_environment! [
    Network;

    PreviewTestNet => "preview-test-net",
    MainTestNet => "main-test-net",
    LocalDevNet => "local-dev-net",
];
