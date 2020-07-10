use crate::config::DEFAULT_HTTP_SERVER;
use clap::{App, Arg};

pub fn cli_app<'a, 'b>() -> App<'a, 'b> {
    App::new("validator_client")
        .visible_aliases(&["v", "vc", "validator"])
        .about(
            "When connected to a beacon node, performs the duties of a staked \
                validator (e.g., proposing blocks and attestations).",
        )
        .arg(
            Arg::with_name("server")
                .long("server")
                .value_name("NETWORK_ADDRESS")
                .help("Address to connect to BeaconNode.")
                .default_value(&DEFAULT_HTTP_SERVER)
                .takes_value(true),
        )
        .arg(Arg::with_name("auto-register").long("auto-register").help(
            "If present, the validator client will register any new signing keys with \
                       the slashing protection database so that they may be used. WARNING: \
                       enabling the same signing key on multiple validator clients WILL lead to \
                       that validator getting slashed. Only use this flag the first time you run \
                       the validator client, or if you're certain there are no other \
                       nodes using the same key. Automatically enabled unless `--strict` is specified",
        ))
        .arg(
            Arg::with_name("allow-unsynced")
                .long("allow-unsynced")
                .help(
                    "If present, the validator client will still poll for duties if the beacon
                      node is not synced.",
                ),
        )
        .arg(
            Arg::with_name("first-validator")
                .value_name("VALIDATOR_INDEX")
                .required(true)
                .help("The first validator public key to be generated for this client."),
        )
        .arg(
            Arg::with_name("last-validator")
                .value_name("VALIDATOR_INDEX")
                .required(true)
                .help("The last validator public key to be generated for this client."),
        )
}
