use eth2_libp2p::{
    multiaddr::{Multiaddr, Protocol},
    Enr,
};
use reqwest::{Error as HttpError, Url};
use std::borrow::Cow;
use std::net::Ipv4Addr;
use types::{BeaconBlock, BeaconState, Checkpoint, EthSpec, Slot};
use url::Host;

#[derive(Debug)]
enum Error {
    InvalidUrl,
    HttpError(HttpError),
}

impl From<HttpError> for Error {
    fn from(e: HttpError) -> Error {
        Error::HttpError(e)
    }
}

/// Used to load "bootstrap" information from the HTTP API of another Lighthouse beacon node.
///
/// Bootstrapping information includes things like genesis and finalized states and blocks, and
/// libp2p connection details.
pub struct Bootstrapper {
    url: Url,
}

impl Bootstrapper {
    /// Parses the given `server` as a URL, instantiating `Self`.
    pub fn from_server_string(server: String) -> Result<Self, String> {
        Ok(Self {
            url: Url::parse(&server).map_err(|e| format!("Invalid bootstrap server url: {}", e))?,
        })
    }

    /// Build a multiaddr using the HTTP server URL that is not guaranteed to be correct.
    ///
    /// The address is created by querying the HTTP server for its listening libp2p addresses.
    /// Then, we find the first TCP port in those addresses and combine the port with the URL of
    /// the server.
    ///
    /// For example, the server `http://192.168.0.1` might end up with a `best_effort_multiaddr` of
    /// `/ipv4/192.168.0.1/tcp/9000` if the server advertises a listening address of
    /// `/ipv4/172.0.0.1/tcp/9000`.
    pub fn best_effort_multiaddr(&self) -> Option<Multiaddr> {
        let tcp_port = self.listen_port().ok()?;

        let mut multiaddr = Multiaddr::with_capacity(2);

        match self.url.host()? {
            Host::Ipv4(addr) => multiaddr.push(Protocol::Ip4(addr)),
            Host::Domain(s) => multiaddr.push(Protocol::Dns4(Cow::Borrowed(s))),
            _ => return None,
        };

        multiaddr.push(Protocol::Tcp(tcp_port));

        Some(multiaddr)
    }

    /// Returns the IPv4 address of the server URL, unless it contains a FQDN.
    pub fn server_ipv4_addr(&self) -> Option<Ipv4Addr> {
        match self.url.host()? {
            Host::Ipv4(addr) => Some(addr),
            _ => None,
        }
    }

    /// Returns the servers ENR address.
    pub fn enr(&self) -> Result<Enr, String> {
        get_enr(self.url.clone()).map_err(|e| format!("Unable to get ENR: {:?}", e))
    }

    /// Returns the servers listening libp2p addresses.
    pub fn listen_port(&self) -> Result<u16, String> {
        get_listen_port(self.url.clone()).map_err(|e| format!("Unable to get listen port: {:?}", e))
    }

    /// Returns the genesis block and state.
    pub fn genesis<T: EthSpec>(&self) -> Result<(BeaconState<T>, BeaconBlock<T>), String> {
        let genesis_slot = Slot::new(0);

        let block = get_block(self.url.clone(), genesis_slot)
            .map_err(|e| format!("Unable to get genesis block: {:?}", e))?;
        let state = get_state(self.url.clone(), genesis_slot)
            .map_err(|e| format!("Unable to get genesis state: {:?}", e))?;

        Ok((state, block))
    }

    /// Returns the most recent finalized state and block.
    pub fn finalized<T: EthSpec>(&self) -> Result<(BeaconState<T>, BeaconBlock<T>), String> {
        let slots_per_epoch = get_slots_per_epoch(self.url.clone())
            .map_err(|e| format!("Unable to get slots per epoch: {:?}", e))?;
        let finalized_slot = get_finalized_slot(self.url.clone(), slots_per_epoch.as_u64())
            .map_err(|e| format!("Unable to get finalized slot: {:?}", e))?;

        let block = get_block(self.url.clone(), finalized_slot)
            .map_err(|e| format!("Unable to get finalized block: {:?}", e))?;
        let state = get_state(self.url.clone(), finalized_slot)
            .map_err(|e| format!("Unable to get finalized state: {:?}", e))?;

        Ok((state, block))
    }
}

fn get_slots_per_epoch(mut url: Url) -> Result<Slot, Error> {
    url.path_segments_mut()
        .map(|mut url| {
            url.push("spec").push("slots_per_epoch");
        })
        .map_err(|_| Error::InvalidUrl)?;

    reqwest::get(url)?
        .error_for_status()?
        .json()
        .map_err(Into::into)
}

fn get_finalized_slot(mut url: Url, slots_per_epoch: u64) -> Result<Slot, Error> {
    url.path_segments_mut()
        .map(|mut url| {
            url.push("beacon").push("latest_finalized_checkpoint");
        })
        .map_err(|_| Error::InvalidUrl)?;

    let checkpoint: Checkpoint = reqwest::get(url)?.error_for_status()?.json()?;

    Ok(checkpoint.epoch.start_slot(slots_per_epoch))
}

fn get_state<T: EthSpec>(mut url: Url, slot: Slot) -> Result<BeaconState<T>, Error> {
    url.path_segments_mut()
        .map(|mut url| {
            url.push("beacon").push("state");
        })
        .map_err(|_| Error::InvalidUrl)?;

    url.query_pairs_mut()
        .append_pair("slot", &format!("{}", slot.as_u64()));

    reqwest::get(url)?
        .error_for_status()?
        .json()
        .map_err(Into::into)
}

fn get_block<T: EthSpec>(mut url: Url, slot: Slot) -> Result<BeaconBlock<T>, Error> {
    url.path_segments_mut()
        .map(|mut url| {
            url.push("beacon").push("block");
        })
        .map_err(|_| Error::InvalidUrl)?;

    url.query_pairs_mut()
        .append_pair("slot", &format!("{}", slot.as_u64()));

    reqwest::get(url)?
        .error_for_status()?
        .json()
        .map_err(Into::into)
}

fn get_enr(mut url: Url) -> Result<Enr, Error> {
    url.path_segments_mut()
        .map(|mut url| {
            url.push("network").push("enr");
        })
        .map_err(|_| Error::InvalidUrl)?;

    reqwest::get(url)?
        .error_for_status()?
        .json()
        .map_err(Into::into)
}

fn get_listen_port(mut url: Url) -> Result<u16, Error> {
    url.path_segments_mut()
        .map(|mut url| {
            url.push("network").push("listen_port");
        })
        .map_err(|_| Error::InvalidUrl)?;

    reqwest::get(url)?
        .error_for_status()?
        .json()
        .map_err(Into::into)
}
