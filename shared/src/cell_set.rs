use ckb_core::block::Block;
use ckb_core::cell::BlockInfo;
use ckb_core::transaction::{CellOutPoint, OutPoint};
use ckb_core::transaction_meta::TransactionMeta;
use ckb_util::{FnvHashMap, FnvHashSet};
use numext_fixed_hash::H256;
use serde_derive::{Deserialize, Serialize};

#[derive(Default, Clone, Deserialize, Serialize)]
pub struct CellSetDiff {
    pub old_inputs: FnvHashSet<OutPoint>,
    pub old_outputs: FnvHashSet<H256>,
    pub new_inputs: FnvHashSet<OutPoint>,
    pub new_outputs: FnvHashMap<H256, (BlockInfo, bool, usize)>,
}

impl CellSetDiff {
    pub fn push_new(&mut self, block: &Block) {
        for tx in block.transactions() {
            let input_iter = tx.input_pts_iter();
            let tx_hash = tx.hash();
            let output_len = tx.outputs().len();
            let block_info = BlockInfo::new(
                block.header().number(),
                block.header().epoch(),
                block.header().parent_hash().clone(),
            );
            self.new_inputs.extend(input_iter.cloned());
            self.new_outputs.insert(
                tx_hash.to_owned(),
                (block_info, tx.is_cellbase(), output_len),
            );
        }
    }

    pub fn push_old(&mut self, block: &Block) {
        for tx in block.transactions() {
            let input_iter = tx.input_pts_iter();
            let tx_hash = tx.hash();

            self.old_inputs.extend(input_iter.cloned());
            self.old_outputs.insert(tx_hash.to_owned());
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CellSetOverlay<'a> {
    origin: &'a FnvHashMap<H256, TransactionMeta>,
    new: FnvHashMap<H256, TransactionMeta>,
    removed: FnvHashSet<H256>,
}

impl<'a> CellSetOverlay<'a> {
    pub fn get(&self, hash: &H256) -> Option<&TransactionMeta> {
        if self.removed.get(hash).is_some() {
            return None;
        }

        self.new.get(hash).or_else(|| self.origin.get(hash))
    }
}

#[derive(Default, Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct CellSet {
    pub(crate) inner: FnvHashMap<H256, TransactionMeta>,
}

pub(crate) enum CellSetOpr {
    Delete,
    Update(TransactionMeta),
}

impl CellSet {
    pub fn new() -> Self {
        CellSet {
            inner: FnvHashMap::default(),
        }
    }

    pub fn new_overlay<'a>(&'a self, diff: &CellSetDiff) -> CellSetOverlay<'a> {
        let mut new = FnvHashMap::default();
        let mut removed = FnvHashSet::default();

        for hash in &diff.old_outputs {
            if self.inner.get(&hash).is_some() {
                removed.insert(hash.clone());
            }
        }

        for (hash, (block_info, cellbase, len)) in diff.new_outputs.clone() {
            removed.remove(&hash);
            if cellbase {
                new.insert(hash, TransactionMeta::new_cellbase(block_info, len, false));
            } else {
                new.insert(hash, TransactionMeta::new(block_info, len, false));
            }
        }

        for old_input in &diff.old_inputs {
            if let Some(cell_input) = &old_input.cell {
                if let Some(meta) = self.inner.get(&cell_input.tx_hash) {
                    let meta = new
                        .entry(cell_input.tx_hash.clone())
                        .or_insert_with(|| meta.clone());
                    meta.unset_dead(cell_input.index as usize);
                }
            }
        }

        for new_input in &diff.new_inputs {
            if let Some(cell_input) = &new_input.cell {
                if let Some(meta) = self.inner.get(&cell_input.tx_hash) {
                    let meta = new
                        .entry(cell_input.tx_hash.clone())
                        .or_insert_with(|| meta.clone());
                    meta.set_dead(cell_input.index as usize);
                }
            }
        }

        CellSetOverlay {
            new,
            removed,
            origin: &self.inner,
        }
    }

    pub fn get(&self, h: &H256) -> Option<&TransactionMeta> {
        self.inner.get(h)
    }

    pub(crate) fn put(&mut self, tx_hash: H256, tx_meta: TransactionMeta) {
        self.inner.insert(tx_hash, tx_meta);
    }

    pub(crate) fn insert_cell(
        &mut self,
        cell: &CellOutPoint,
        number: u64,
        epoch: u64,
        parent: H256,
        cellbase: bool,
        outputs_len: usize,
    ) -> TransactionMeta {
        let block_info = BlockInfo::new(number, epoch, parent);
        let mut meta = if cellbase {
            TransactionMeta::new_cellbase(block_info, outputs_len, true)
        } else {
            TransactionMeta::new(block_info, outputs_len, true)
        };
        meta.unset_dead(cell.index as usize);
        self.inner.insert(cell.tx_hash.clone(), meta.clone());
        meta
    }

    pub(crate) fn insert_transaction(
        &mut self,
        tx_hash: H256,
        block_info: BlockInfo,
        cellbase: bool,
        outputs_len: usize,
    ) -> TransactionMeta {
        let meta = if cellbase {
            TransactionMeta::new_cellbase(block_info, outputs_len, false)
        } else {
            TransactionMeta::new(block_info, outputs_len, false)
        };
        self.inner.insert(tx_hash, meta.clone());
        meta
    }

    pub(crate) fn remove(&mut self, tx_hash: &H256) -> Option<TransactionMeta> {
        self.inner.remove(tx_hash)
    }

    pub(crate) fn mark_dead(&mut self, cell: &CellOutPoint) -> Option<CellSetOpr> {
        self.inner.get_mut(&cell.tx_hash).map(|meta| {
            meta.set_dead(cell.index as usize);
            if meta.all_dead() {
                CellSetOpr::Delete
            } else {
                CellSetOpr::Update(meta.clone())
            }
        })
    }

    // if we aleady removed the cell, `mark` will return None, else return the meta
    pub(crate) fn try_mark_live(&mut self, cell: &CellOutPoint) -> Option<TransactionMeta> {
        if let Some(meta) = self.inner.get_mut(&cell.tx_hash) {
            meta.unset_dead(cell.index as usize);
            Some(meta.clone())
        } else {
            None
        }
    }
}
