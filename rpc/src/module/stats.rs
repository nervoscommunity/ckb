use ckb_network_alert::notifier::Notifier as AlertNotifier;
use ckb_shared::shared::Shared;
use ckb_store::ChainStore;
use ckb_sync::Synchronizer;
use ckb_traits::BlockMedianTimeContext;
use ckb_util::Mutex;
use jsonrpc_core::Result;
use jsonrpc_derive::rpc;
use jsonrpc_types::{AlertMessage, ChainInfo, EpochNumber, PeerState, Timestamp};
use std::sync::Arc;

#[rpc]
pub trait StatsRpc {
    #[rpc(name = "get_blockchain_info")]
    fn get_blockchain_info(&self) -> Result<ChainInfo>;

    #[rpc(name = "get_peers_state")]
    fn get_peers_state(&self) -> Result<Vec<PeerState>>;
}

pub(crate) struct StatsRpcImpl<CS>
where
    CS: ChainStore,
{
    pub shared: Shared<CS>,
    pub synchronizer: Synchronizer<CS>,
    pub alert_notifier: Arc<Mutex<AlertNotifier>>,
}

impl<CS: ChainStore + 'static> StatsRpc for StatsRpcImpl<CS> {
    fn get_blockchain_info(&self) -> Result<ChainInfo> {
        let chain = self.synchronizer.shared.consensus().id.clone();
        let (tip_header, median_time) = {
            let chain_state = self.shared.lock_chain_state();
            let tip_header = chain_state.tip_header().clone();
            let median_time =
                (&*chain_state).block_median_time(tip_header.number(), tip_header.hash());
            (tip_header, median_time)
        };
        let epoch = tip_header.epoch();
        let difficulty = tip_header.difficulty().clone();
        let is_initial_block_download = self.synchronizer.shared.is_initial_block_download();
        let alerts: Vec<AlertMessage> = {
            let now = faketime::unix_time_as_millis();
            let mut notifier = self.alert_notifier.lock();
            notifier.clear_expired_alerts(now);
            notifier
                .noticed_alerts()
                .iter()
                .map(|alert| AlertMessage::from(alert.as_ref()))
                .collect()
        };

        Ok(ChainInfo {
            chain,
            median_time: Timestamp(median_time),
            epoch: EpochNumber(epoch),
            difficulty,
            is_initial_block_download,
            alerts,
        })
    }

    fn get_peers_state(&self) -> Result<Vec<PeerState>> {
        // deprecated
        Ok(self
            .synchronizer
            .shared()
            .read_inflight_blocks()
            .blocks_iter()
            .map(|(peer, blocks)| PeerState::new(peer.value(), 0, blocks.len()))
            .collect())
    }
}
