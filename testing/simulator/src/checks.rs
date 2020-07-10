use crate::local_network::LocalNetwork;
use std::time::Duration;
use types::{Epoch, EthSpec};

/// Checks that the chain has made the first possible finalization.
///
/// Intended to be run as soon as chain starts.
pub async fn verify_first_finalization<E: EthSpec>(
    network: LocalNetwork<E>,
    slot_duration: Duration,
) -> Result<(), String> {
    epoch_delay(Epoch::new(4), slot_duration, E::slots_per_epoch()).await;
    verify_all_finalized_at(network, Epoch::new(2)).await?;
    Ok(())
}

/// Delays for `epochs`, plus half a slot extra.
pub async fn epoch_delay(epochs: Epoch, slot_duration: Duration, slots_per_epoch: u64) {
    let duration = slot_duration * (epochs.as_u64() * slots_per_epoch) as u32 + slot_duration / 2;
    tokio::time::delay_for(duration).await
}

/// Verifies that all beacon nodes in the given network have a head state that has a finalized
/// epoch of `epoch`.
pub async fn verify_all_finalized_at<E: EthSpec>(
    network: LocalNetwork<E>,
    epoch: Epoch,
) -> Result<(), String> {
    let epochs = {
        let mut epochs = Vec::new();
        for remote_node in network.remote_nodes()? {
            epochs.push(
                remote_node
                    .http
                    .beacon()
                    .get_head()
                    .await
                    .map(|head| head.finalized_slot.epoch(E::slots_per_epoch()))
                    .map_err(|e| format!("Get head via http failed: {:?}", e))?,
            );
        }
        epochs
    };

    if epochs.iter().any(|node_epoch| *node_epoch != epoch) {
        Err(format!(
            "Nodes are not finalized at epoch {}. Finalized epochs: {:?}",
            epoch, epochs
        ))
    } else {
        Ok(())
    }
}
