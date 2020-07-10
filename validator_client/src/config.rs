use clap::ArgMatches;
use clap_utils::{parse_optional, parse_path_with_default_in_home_dir, parse_required};
use serde_derive::{Deserialize, Serialize};
use std::path::PathBuf;

pub const DEFAULT_HTTP_SERVER: &str = "http://localhost:5052/";
pub const DEFAULT_DATA_DIR: &str = ".lighthouse/validators";
/// Path to the slashing protection database within the datadir.
pub const SLASHING_PROTECTION_FILENAME: &str = "slashing_protection.sqlite";

/// Stores the core configuration for this validator instance.
#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    /// The data directory, which stores all validator databases
    pub data_dir: PathBuf,
    /// The http endpoint of the beacon node API.
    ///
    /// Should be similar to `http://localhost:8080`
    pub http_server: String,
    /// If true, the validator client will still poll for duties and produce blocks even if the
    /// beacon node is not synced at startup.
    pub allow_unsynced_beacon_node: bool,
    /// If true, register new validator keys with the slashing protection database.
    pub auto_register: bool,

    pub first_validator: usize,
    pub last_validator: usize,
}

impl Default for Config {
    /// Build a new configuration from defaults.
    fn default() -> Self {
        let data_dir = dirs::home_dir()
            .map(|home| home.join(DEFAULT_DATA_DIR))
            .unwrap_or_else(|| PathBuf::from("."));
        Self {
            data_dir,
            http_server: DEFAULT_HTTP_SERVER.to_string(),
            allow_unsynced_beacon_node: false,
            auto_register: false,

            first_validator: 0,
            last_validator: 0,
        }
    }
}

impl Config {
    /// Returns a `Default` implementation of `Self` with some parameters modified by the supplied
    /// `cli_args`.
    pub fn from_cli(cli_args: &ArgMatches) -> Result<Config, String> {
        let mut config = Config::default();

        config.data_dir = parse_path_with_default_in_home_dir(
            cli_args,
            "datadir",
            PathBuf::from(".lighthouse").join("validators"),
        )?;

        if !config.data_dir.exists() {
            return Err(format!(
                "The directory for validator data  (--datadir) does not exist: {:?}",
                config.data_dir
            ));
        }

        if let Some(server) = parse_optional(cli_args, "server")? {
            config.http_server = server;
        }

        config.allow_unsynced_beacon_node = cli_args.is_present("allow-unsynced");
        config.auto_register = cli_args.is_present("auto-register");

        config.first_validator = parse_required(cli_args, "first-validator")?;
        config.last_validator = parse_required(cli_args, "last-validator")?;

        Ok(config)
    }
}
