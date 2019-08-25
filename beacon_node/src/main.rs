mod config;
mod run;

use clap::{App, Arg, SubCommand};
use config::get_configs;
use env_logger::{Builder, Env};
use slog::{crit, o, warn, Drain, Level};

pub const DEFAULT_DATA_DIR: &str = ".lighthouse";

pub const CLIENT_CONFIG_FILENAME: &str = "beacon-node.toml";
pub const ETH2_CONFIG_FILENAME: &str = "eth2-spec.toml";
pub const TESTNET_CONFIG_FILENAME: &str = "testnet.toml";

fn main() {
    // debugging output for libp2p and external crates
    Builder::from_env(Env::default()).init();

    let matches = App::new("Lighthouse")
        .version(version::version().as_str())
        .author("Sigma Prime <contact@sigmaprime.io>")
        .about("Eth 2.0 Client")
        /*
         * Configuration directory locations.
         */
        .arg(
            Arg::with_name("datadir")
                .long("datadir")
                .value_name("DIR")
                .help("Data directory for keys and databases.")
                .takes_value(true)
                .global(true)
        )
        .arg(
            Arg::with_name("logfile")
                .long("logfile")
                .value_name("logfile")
                .help("File path where output will be written.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("network-dir")
                .long("network-dir")
                .value_name("NETWORK-DIR")
                .help("Data directory for network keys.")
                .takes_value(true)
                .global(true)
        )
        /*
         * Network parameters.
         */
        .arg(
            Arg::with_name("listen-address")
                .long("listen-address")
                .value_name("ADDRESS")
                .help("The address lighthouse will listen for UDP and TCP connections. (default 127.0.0.1).")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("port")
                .long("port")
                .value_name("PORT")
                .help("The TCP/UDP port to listen on. The UDP port can be modified by the --discovery-port flag.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("maxpeers")
                .long("maxpeers")
                .help("The maximum number of peers (default 10).")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("boot-nodes")
                .long("boot-nodes")
                .allow_hyphen_values(true)
                .value_name("BOOTNODES")
                .help("One or more comma-delimited base64-encoded ENR's to bootstrap the p2p network.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("discovery-port")
                .long("disc-port")
                .value_name("PORT")
                .help("The discovery UDP port.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("discovery-address")
                .long("discovery-address")
                .value_name("ADDRESS")
                .help("The IP address to broadcast to other peers on how to reach this node.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("topics")
                .long("topics")
                .value_name("STRING")
                .help("One or more comma-delimited gossipsub topic strings to subscribe to.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("libp2p-addresses")
                .long("libp2p-addresses")
                .value_name("MULTIADDR")
                .help("One or more comma-delimited multiaddrs to manually connect to a libp2p peer without an ENR.")
                .takes_value(true),
        )
        /*
         * gRPC parameters.
         */
        .arg(
            Arg::with_name("rpc")
                .long("rpc")
                .value_name("RPC")
                .help("Enable the RPC server.")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("rpc-address")
                .long("rpc-address")
                .value_name("Address")
                .help("Listen address for RPC endpoint.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("rpc-port")
                .long("rpc-port")
                .help("Listen port for RPC endpoint.")
                .takes_value(true),
        )
        /* Client related arguments */
        .arg(
            Arg::with_name("api")
                .long("api")
                .value_name("API")
                .help("Enable the RESTful HTTP API server.")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("api-address")
                .long("api-address")
                .value_name("APIADDRESS")
                .help("Set the listen address for the RESTful HTTP API server.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("api-port")
                .long("api-port")
                .value_name("APIPORT")
                .help("Set the listen TCP port for the RESTful HTTP API server.")
                .takes_value(true),
        )

        /*
         * Database parameters.
         */
        .arg(
            Arg::with_name("db")
                .long("db")
                .value_name("DB")
                .help("Type of database to use.")
                .takes_value(true)
                .possible_values(&["disk", "memory"])
                .default_value("disk"),
        )
        /*
         * Logging.
         */
        .arg(
            Arg::with_name("debug-level")
                .long("debug-level")
                .value_name("LEVEL")
                .help("The title of the spec constants for chain config.")
                .takes_value(true)
                .possible_values(&["info", "debug", "trace", "warn", "error", "crit"])
                .default_value("trace"),
        )
        .arg(
            Arg::with_name("verbosity")
                .short("v")
                .multiple(true)
                .help("Sets the verbosity level")
                .takes_value(true),
        )
        /*
         * The "testnet" sub-command.
         *
         * Allows for creating a new datadir with testnet-specific configs.
         */
        .subcommand(SubCommand::with_name("testnet")
            .about("Create a new Lighthouse datadir using a testnet strategy.")
            .arg(
                Arg::with_name("spec")
                    .short("s")
                    .long("spec")
                    .value_name("TITLE")
                    .help("Specifies the default eth2 spec type. Only effective when creating a new datadir.")
                    .takes_value(true)
                    .required(true)
                    .possible_values(&["mainnet", "minimal", "interop"])
            )
            .arg(
                Arg::with_name("random-datadir")
                    .long("random-datadir")
                    .short("r")
                    .help("If present, append a random string to the datadir path. Useful for fast development \
                          iteration.")
            )
            .arg(
                Arg::with_name("force")
                    .long("force")
                    .short("f")
                    .help("If present, will backup any existing config files before creating new ones. Cannot be \
                          used when specifying --random-datadir (logic error).")
                    .conflicts_with("random-datadir")
            )
            /*
             * Testnet sub-commands.
             *
             * `boostrap`
             *
             * Start a new node by downloading genesis and network info from another node via the
             * HTTP API.
             */
            .subcommand(SubCommand::with_name("bootstrap")
                .about("Connects to the given HTTP server, downloads a genesis state and attempts to peer with it.")
                .arg(Arg::with_name("server")
                    .value_name("HTTP_SERVER")
                    .required(true)
                    .help("A HTTP server, with a http:// prefix"))
                .arg(Arg::with_name("libp2p-port")
                    .short("p")
                    .long("port")
                    .value_name("TCP_PORT")
                    .help("A libp2p listen port used to peer with the bootstrap server. This flag is useful \
                           when port-fowarding is used: you may connect using a different port than \
                           the one the server is immediately listening on."))
            )
            /*
             * `recent`
             *
             * Start a new node, with a specified number of validators with a genesis time in the last
             * 30-minutes.
             */
            .subcommand(SubCommand::with_name("recent")
                .about("Creates a new genesis state where the genesis time was at the previous \
                       30-minute boundary (e.g., 12:00, 12:30, 13:00, etc.)")
                .arg(Arg::with_name("validator_count")
                    .value_name("VALIDATOR_COUNT")
                    .required(true)
                    .help("The number of validators in the genesis state"))
            )
            .subcommand(SubCommand::with_name("yaml-genesis-state")
                .about("Creates a new datadir where the genesis state is read from YAML. Will fail to parse \
                       a YAML state that was generated to a different spec than that specified by --spec.")
                .arg(Arg::with_name("file")
                    .value_name("YAML_FILE")
                    .required(true)
                    .help("A YAML file from which to read the state"))
            )
        )
        .get_matches();

    // build the initial logger
    let decorator = slog_term::TermDecorator::new().build();
    let decorator = logging::AlignedTermDecorator::new(decorator, logging::MAX_MESSAGE_WIDTH);
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build();

    let drain = match matches.value_of("debug-level") {
        Some("info") => drain.filter_level(Level::Info),
        Some("debug") => drain.filter_level(Level::Debug),
        Some("trace") => drain.filter_level(Level::Trace),
        Some("warn") => drain.filter_level(Level::Warning),
        Some("error") => drain.filter_level(Level::Error),
        Some("crit") => drain.filter_level(Level::Critical),
        _ => unreachable!("guarded by clap"),
    };

    let drain = match matches.occurrences_of("verbosity") {
        0 => drain.filter_level(Level::Info),
        1 => drain.filter_level(Level::Debug),
        2 => drain.filter_level(Level::Trace),
        _ => drain.filter_level(Level::Trace),
    };

    let log = slog::Logger::root(drain.fuse(), o!());

    warn!(
        log,
        "Ethereum 2.0 is pre-release. This software is experimental."
    );

    // Load the process-wide configuration.
    //
    // May load this from disk or create a new configuration, depending on the CLI flags supplied.
    let (client_config, eth2_config) = match get_configs(&matches, &log) {
        Ok(configs) => configs,
        Err(e) => {
            crit!(log, "Failed to load configuration"; "error" => e);
            return;
        }
    };

    // Start the node using a `tokio` executor.
    match run::run_beacon_node(client_config, eth2_config, &log) {
        Ok(_) => {}
        Err(e) => crit!(log, "Beacon node failed to start"; "reason" => format!("{:}", e)),
    }
}
