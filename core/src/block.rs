use crate::header::{Header, HeaderBuilder};
use crate::transaction::{ProposalShortId, Transaction};
use crate::uncle::{uncles_hash, UncleBlock};
use crate::Capacity;
use ckb_merkle_tree::merkle_root;
use fnv::FnvHashSet;
use hash::new_blake2b;
use numext_fixed_hash::H256;
use serde_derive::{Deserialize, Serialize};
use std::borrow::ToOwned;
use std::iter::FromIterator;
use std::ops::Deref;

fn cal_transactions_root(vec: &[Transaction]) -> H256 {
    merkle_root(
        &vec.iter()
            .map(|tx| tx.hash().to_owned())
            .collect::<Vec<_>>(),
    )
}

fn cal_witnesses_root(vec: &[Transaction]) -> H256 {
    merkle_root(
        &vec.iter()
            .map(|tx| tx.witness_hash().to_owned())
            .collect::<Vec<_>>(),
    )
}

pub(crate) fn cal_proposals_hash(vec: &[ProposalShortId]) -> H256 {
    if vec.is_empty() {
        H256::zero()
    } else {
        let mut ret = [0u8; 32];
        let mut blake2b = new_blake2b();
        for id in vec.iter() {
            blake2b.update(id.deref());
        }
        blake2b.finalize(&mut ret);
        ret.into()
    }
}

#[derive(Clone, Serialize, Deserialize, Eq, Debug)]
pub struct Block {
    header: Header,
    uncles: Vec<UncleBlock>,
    transactions: Vec<Transaction>,
    proposals: Vec<ProposalShortId>,
}

impl Block {
    pub fn new(
        header: Header,
        uncles: Vec<UncleBlock>,
        transactions: Vec<Transaction>,
        proposals: Vec<ProposalShortId>,
    ) -> Block {
        Block {
            header,
            uncles,
            transactions,
            proposals,
        }
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

    pub fn is_genesis(&self) -> bool {
        self.header.is_genesis()
    }

    pub fn transactions(&self) -> &[Transaction] {
        &self.transactions
    }

    pub fn cellbase(&self) -> &Transaction {
        &self.transactions.get(0).expect("get cellbase transaction")
    }

    pub fn proposals(&self) -> &[ProposalShortId] {
        &self.proposals
    }

    pub fn uncles(&self) -> &[UncleBlock] {
        &self.uncles
    }

    pub fn cal_uncles_hash(&self) -> H256 {
        uncles_hash(&self.uncles)
    }

    pub fn union_proposal_ids(&self) -> FnvHashSet<ProposalShortId> {
        let mut ids = FnvHashSet::from_iter(self.proposals().to_owned());

        for uc in &self.uncles {
            ids.extend(uc.proposals());
        }

        ids
    }

    pub fn cal_witnesses_root(&self) -> H256 {
        cal_witnesses_root(self.transactions())
    }

    pub fn cal_transactions_root(&self) -> H256 {
        cal_transactions_root(self.transactions())
    }

    pub fn cal_proposals_hash(&self) -> H256 {
        cal_proposals_hash(self.proposals())
    }

    pub fn serialized_size(&self, proof_size: usize) -> usize {
        Header::serialized_size(proof_size)
            + self
                .uncles
                .iter()
                .map(|u| u.serialized_size(proof_size))
                .sum::<usize>()
            + self.proposals.len() * ProposalShortId::serialized_size()
            + self
                .transactions()
                .iter()
                .map(Transaction::serialized_size)
                .sum::<usize>()
    }

    pub fn outputs_capacity(&self) -> ::occupied_capacity::Result<Capacity> {
        self.transactions
            .iter()
            .try_fold(Capacity::zero(), |capacity, tx| {
                tx.outputs_capacity().and_then(|c| capacity.safe_add(c))
            })
    }
}

impl ::std::hash::Hash for Block {
    fn hash<H>(&self, state: &mut H)
    where
        H: ::std::hash::Hasher,
    {
        state.write(&self.header.hash().as_bytes());
        state.finish();
    }
}

impl PartialEq for Block {
    fn eq(&self, other: &Block) -> bool {
        self.header().hash() == other.header().hash()
    }
}

#[derive(Default)]
pub struct BlockBuilder {
    header_builder: HeaderBuilder,
    uncles: Vec<UncleBlock>,
    transactions: Vec<Transaction>,
    proposals: Vec<ProposalShortId>,
}

impl BlockBuilder {
    pub fn from_block(block: Block) -> Self {
        let Block {
            header,
            uncles,
            transactions,
            proposals,
        } = block;
        Self {
            header_builder: HeaderBuilder::from_header(header),
            uncles,
            transactions,
            proposals,
        }
    }

    pub fn from_header_builder(header_builder: HeaderBuilder) -> Self {
        Self {
            header_builder,
            uncles: Vec::new(),
            transactions: Vec::new(),
            proposals: Vec::new(),
        }
    }

    pub fn header_builder(mut self, header_builder: HeaderBuilder) -> Self {
        self.header_builder = header_builder;
        self
    }

    pub fn header(mut self, header: Header) -> Self {
        self.header_builder = HeaderBuilder::from_header(header);
        self
    }

    pub fn uncle(mut self, uncle: UncleBlock) -> Self {
        self.uncles.push(uncle);
        self
    }

    pub fn uncles(mut self, uncles: Vec<UncleBlock>) -> Self {
        self.uncles.extend(uncles);
        self
    }

    pub fn transaction(mut self, transaction: Transaction) -> Self {
        self.transactions.push(transaction);
        self
    }

    pub fn transactions(mut self, transactions: Vec<Transaction>) -> Self {
        self.transactions.extend(transactions);
        self
    }

    pub fn proposal(mut self, proposal_short_id: ProposalShortId) -> Self {
        self.proposals.push(proposal_short_id);
        self
    }

    pub fn proposals(mut self, proposal_short_ids: Vec<ProposalShortId>) -> Self {
        self.proposals.extend(proposal_short_ids);
        self
    }

    /// # Warning
    ///
    /// For testing purpose only, this method is used to construct a incorrect Block.
    pub unsafe fn build_unchecked(self) -> Block {
        let Self {
            header_builder,
            uncles,
            transactions,
            proposals,
        } = self;
        Block {
            header: header_builder.build(),
            uncles,
            transactions,
            proposals,
        }
    }

    pub fn build(self) -> Block {
        let Self {
            header_builder,
            uncles,
            transactions,
            proposals,
        } = self;
        let transactions_root = cal_transactions_root(&transactions);
        let witnesses_root = cal_witnesses_root(&transactions);
        let proposals_hash = cal_proposals_hash(&proposals);
        let uncles_hash = uncles_hash(&uncles);
        let header = header_builder
            .transactions_root(transactions_root)
            .witnesses_root(witnesses_root)
            .proposals_hash(proposals_hash)
            .uncles_hash(uncles_hash)
            .uncles_count(uncles.len() as u32)
            .build();
        Block {
            header,
            uncles,
            transactions,
            proposals,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use numext_fixed_hash::H256;

    #[test]
    fn test_cal_proposals_hash() {
        let proposal1 = ProposalShortId::new([1; 10]);
        let proposal2 = ProposalShortId::new([2; 10]);
        let proposals = [proposal1, proposal2];

        let id = cal_proposals_hash(&proposals);

        assert_eq!(
            id,
            H256([
                0xd1, 0x67, 0x0e, 0x45, 0xaf, 0x1d, 0xeb, 0x9c, 0xc0, 0x09, 0x51, 0xd7, 0x1c, 0x09,
                0xce, 0x80, 0x93, 0x2e, 0x7d, 0xdf, 0x9f, 0xb1, 0x51, 0xd7, 0x44, 0x43, 0x6b, 0xd0,
                0x4a, 0xc4, 0xa5, 0x62
            ])
        );
    }
}
