use crate::cell::BlockInfo;
use bit_vec::BitVec;
use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(remote = "BitVec")]
struct BitVecSerde {
    #[serde(getter = "BitVec::to_bytes")]
    bits: Vec<u8>,
    #[serde(getter = "BitVec::len")]
    len: usize,
}

impl From<BitVecSerde> for BitVec {
    fn from(bv: BitVecSerde) -> BitVec {
        let mut bit_vec = BitVec::from_bytes(&bv.bits);
        bit_vec.truncate(bv.len);
        bit_vec
    }
}

#[derive(Default, Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct TransactionMeta {
    block_info: BlockInfo,
    cellbase: bool,
    /// each bits indicate if transaction has dead cells
    #[serde(with = "BitVecSerde")]
    dead_cell: BitVec,
}

impl TransactionMeta {
    pub fn new(block_info: BlockInfo, outputs_count: usize, all_dead: bool) -> TransactionMeta {
        TransactionMeta {
            block_info,
            cellbase: false,
            dead_cell: BitVec::from_elem(outputs_count, all_dead),
        }
    }

    /// New cellbase transaction
    pub fn new_cellbase(block_info: BlockInfo, outputs_count: usize, all_dead: bool) -> Self {
        let mut result = Self::new(block_info, outputs_count, all_dead);
        result.cellbase = true;
        result
    }

    /// Returns true if it is a cellbase transaction
    pub fn is_cellbase(&self) -> bool {
        self.cellbase
    }

    /// Returns transaction outputs count
    pub fn len(&self) -> usize {
        self.dead_cell.len()
    }

    pub fn block_number(&self) -> u64 {
        self.block_info.number
    }

    pub fn epoch_number(&self) -> u64 {
        self.block_info.epoch
    }

    pub fn is_empty(&self) -> bool {
        self.dead_cell.is_empty()
    }

    pub fn is_dead(&self, index: usize) -> Option<bool> {
        self.dead_cell.get(index)
    }

    pub fn all_dead(&self) -> bool {
        self.dead_cell.all()
    }

    pub fn set_dead(&mut self, index: usize) {
        if index < self.len() {
            self.dead_cell.set(index, true);
        }
    }

    pub fn unset_dead(&mut self, index: usize) {
        if index < self.len() {
            self.dead_cell.set(index, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bincode;

    #[test]
    fn transaction_meta_serde() {
        let mut original = TransactionMeta::new(BlockInfo::new(0, 0), 4, false);
        original.set_dead(1);
        original.set_dead(3);

        let decoded: TransactionMeta =
            bincode::deserialize(&(bincode::serialize(&original).unwrap())[..]).unwrap();

        assert!(decoded.is_dead(0) == Some(false));
        assert!(decoded.is_dead(1) == Some(true));
        assert!(decoded.is_dead(2) == Some(false));
        assert!(decoded.is_dead(3) == Some(true));
        assert!(decoded.is_dead(4) == None);
    }

    #[test]
    fn set_unset_dead_out_of_bounds() {
        let mut meta = TransactionMeta::new(BlockInfo::new(0, 0), 4, false);
        meta.set_dead(3);
        assert!(meta.is_dead(3) == Some(true));
        meta.unset_dead(3);
        assert!(meta.is_dead(3) == Some(false));
        // none-op
        meta.set_dead(4);
        assert!(meta.is_dead(4) == None);
        meta.unset_dead(4);
        assert!(meta.is_dead(4) == None);
    }
}
