use crate::{BlockNumber, Capacity, EpochNumber};
use failure::Error as FailureError;
use numext_fixed_hash::H256;
use numext_fixed_uint::U256;
use serde_derive::{Deserialize, Serialize};

pub const DEFAULT_ACCUMULATED_RATE: u64 = 10_000_000_000_000_000;

#[derive(Clone, Serialize, Deserialize, PartialEq, Default, Debug)]
pub struct BlockExt {
    pub received_at: u64,
    pub total_difficulty: U256,
    pub total_uncles_count: u64,
    pub verified: Option<bool>,
    pub dao_stats: DaoStats,
    pub txs_fees: Vec<Capacity>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Default, Debug)]
pub struct DaoStats {
    // DAO related fields
    // accmulated rate is multiplied by 10**16 to keep as much decimals
    // as we can. In this case, we can still represent a maximum value
    // of around 1844.6744073709551, which is good enough for our case.
    pub accumulated_rate: u64,
    pub accumulated_capacity: u64,
}

#[derive(Clone, Serialize, Deserialize, Eq, PartialEq, Debug)]
pub struct TransactionAddress {
    // Block hash
    pub block_hash: H256,
    // Offset of block transaction in serialized bytes
    pub offset: usize,
    pub length: usize,
}

#[derive(Clone, Serialize, Deserialize, Eq, PartialEq, Debug)]
pub struct EpochExt {
    pub(crate) number: EpochNumber,
    pub(crate) block_reward: Capacity,
    pub(crate) remainder_reward: Capacity,
    pub(crate) last_block_hash_in_previous_epoch: H256,
    pub(crate) start_number: BlockNumber,
    pub(crate) length: BlockNumber,
    pub(crate) difficulty: U256,
}

impl EpochExt {
    pub fn number(&self) -> u64 {
        self.number
    }

    pub fn block_reward(&self, number: BlockNumber) -> Result<Capacity, FailureError> {
        if number >= self.start_number()
            && number < self.start_number() + self.remainder_reward.as_u64()
        {
            self.block_reward
                .safe_add(Capacity::one())
                .map_err(Into::into)
        } else {
            Ok(self.block_reward)
        }
    }

    pub fn is_genesis(&self) -> bool {
        0 == self.number
    }

    pub fn start_number(&self) -> BlockNumber {
        self.start_number
    }

    pub fn length(&self) -> BlockNumber {
        self.length
    }

    pub fn set_length(&mut self, length: BlockNumber) {
        self.length = length;
    }

    pub fn set_difficulty(&mut self, difficulty: U256) {
        self.difficulty = difficulty;
    }

    pub fn difficulty(&self) -> &U256 {
        &self.difficulty
    }

    pub fn remainder_reward(&self) -> &Capacity {
        &self.remainder_reward
    }

    pub fn last_block_hash_in_previous_epoch(&self) -> &H256 {
        &self.last_block_hash_in_previous_epoch
    }

    pub fn new(
        number: u64,
        block_reward: Capacity,
        remainder_reward: Capacity,
        last_block_hash_in_previous_epoch: H256,
        start_number: BlockNumber,
        length: BlockNumber,
        difficulty: U256,
    ) -> EpochExt {
        EpochExt {
            number,
            block_reward,
            remainder_reward,
            start_number,
            last_block_hash_in_previous_epoch,
            length,
            difficulty,
        }
    }

    pub fn destruct(
        self,
    ) -> (
        u64,
        Capacity,
        Capacity,
        H256,
        BlockNumber,
        BlockNumber,
        U256,
    ) {
        let EpochExt {
            number,
            block_reward,
            remainder_reward,
            start_number,
            last_block_hash_in_previous_epoch,
            length,
            difficulty,
        } = self;
        (
            number,
            block_reward,
            remainder_reward,
            last_block_hash_in_previous_epoch,
            start_number,
            length,
            difficulty,
        )
    }
}
