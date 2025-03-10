use crate::block::Block;
use crate::header::Header;
use crate::transaction::{CellOutPoint, CellOutput, OutPoint, Transaction};
use crate::Capacity;
use crate::{BlockNumber, EpochNumber};
use ckb_util::LowerHexOption;
use fnv::{FnvHashMap, FnvHashSet};
use numext_fixed_hash::H256;
use serde_derive::{Deserialize, Serialize};
use std::convert::AsRef;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq, Default, Deserialize, Serialize)]
pub struct BlockInfo {
    pub number: BlockNumber,
    pub epoch: EpochNumber,
}

impl BlockInfo {
    pub fn new(number: BlockNumber, epoch: EpochNumber) -> Self {
        BlockInfo { number, epoch }
    }
}

#[derive(Clone, Eq, PartialEq, Default, Deserialize, Serialize)]
pub struct CellMeta {
    #[serde(skip)]
    pub cell_output: Option<CellOutput>,
    pub out_point: CellOutPoint,
    pub block_info: Option<BlockInfo>,
    pub cellbase: bool,
    pub capacity: Capacity,
    pub data_hash: Option<H256>,
}

#[derive(Default)]
pub struct CellMetaBuilder {
    cell_output: Option<CellOutput>,
    out_point: CellOutPoint,
    block_info: Option<BlockInfo>,
    cellbase: bool,
    capacity: Capacity,
    data_hash: Option<H256>,
}

impl CellMetaBuilder {
    pub fn from_cell_meta(cell_meta: CellMeta) -> Self {
        let CellMeta {
            cell_output,
            out_point,
            block_info,
            cellbase,
            capacity,
            data_hash,
        } = cell_meta;
        Self {
            cell_output,
            out_point,
            block_info,
            cellbase,
            capacity,
            data_hash,
        }
    }

    pub fn from_cell_output(cell_output: CellOutput) -> Self {
        CellMetaBuilder::default()
            .capacity(cell_output.capacity)
            .cell_output(cell_output)
    }

    pub fn cell_output(mut self, cell_output: CellOutput) -> Self {
        self.cell_output = Some(cell_output);
        self
    }

    pub fn out_point(mut self, out_point: CellOutPoint) -> Self {
        self.out_point = out_point;
        self
    }

    pub fn block_info(mut self, block_info: BlockInfo) -> Self {
        self.block_info = Some(block_info);
        self
    }

    pub fn cellbase(mut self, cellbase: bool) -> Self {
        self.cellbase = cellbase;
        self
    }

    pub fn capacity(mut self, capacity: Capacity) -> Self {
        self.capacity = capacity;
        self
    }

    pub fn data_hash(mut self, data_hash: H256) -> Self {
        self.data_hash = Some(data_hash);
        self
    }

    pub fn build(self) -> CellMeta {
        let Self {
            cell_output,
            out_point,
            block_info,
            cellbase,
            capacity,
            data_hash,
        } = self;
        CellMeta {
            cell_output,
            out_point,
            block_info,
            cellbase,
            capacity,
            data_hash,
        }
    }
}

impl From<&CellOutput> for CellMeta {
    fn from(output: &CellOutput) -> Self {
        CellMetaBuilder::from_cell_output(output.to_owned()).build()
    }
}

impl fmt::Debug for CellMeta {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("CellMeta")
            .field("cell_output", &self.cell_output)
            .field("out_point", &self.out_point)
            .field("block_info", &self.block_info)
            .field("cellbase", &self.cellbase)
            .field("capacity", &self.capacity)
            .field(
                "data_hash",
                &format_args!("{:#x}", LowerHexOption(self.data_hash.as_ref())),
            )
            .finish()
    }
}

impl CellMeta {
    pub fn is_cellbase(&self) -> bool {
        self.cellbase
    }

    pub fn capacity(&self) -> Capacity {
        self.capacity
    }

    pub fn data_hash(&self) -> Option<&H256> {
        self.data_hash.as_ref()
    }
}

#[derive(PartialEq, Debug)]
pub enum CellStatus {
    /// Cell exists and has not been spent.
    Live(Box<CellMeta>),
    /// Cell exists and has been spent.
    Dead,
    /// Cell does not exist.
    Unknown,
    /// OutPoint doesn't contain reference to a cell.
    Unspecified,
}

impl CellStatus {
    pub fn live_cell(cell_meta: CellMeta) -> CellStatus {
        CellStatus::Live(Box::new(cell_meta))
    }

    pub fn is_live(&self) -> bool {
        match *self {
            CellStatus::Live(_) => true,
            _ => false,
        }
    }

    pub fn is_dead(&self) -> bool {
        self == &CellStatus::Dead
    }

    pub fn is_unknown(&self) -> bool {
        self == &CellStatus::Unknown
    }

    pub fn is_unspecified(&self) -> bool {
        self == &CellStatus::Unspecified
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum HeaderStatus {
    /// Header exists on current chain
    Live(Box<Header>),
    /// Header exists, but the specified block doesn't contain referenced transaction.
    InclusionFaliure,
    /// Header does not exist on current chain
    Unknown,
    /// OutPoint doesn't contain reference to a header.
    Unspecified,
}

impl HeaderStatus {
    pub fn live_header(header: Header) -> HeaderStatus {
        HeaderStatus::Live(Box::new(header))
    }

    pub fn is_live(&self) -> bool {
        match *self {
            HeaderStatus::Live(_) => true,
            _ => false,
        }
    }

    pub fn is_inclusion_failure(&self) -> bool {
        self == &HeaderStatus::InclusionFaliure
    }

    pub fn is_unknown(&self) -> bool {
        self == &HeaderStatus::Unknown
    }

    pub fn is_unspecified(&self) -> bool {
        self == &HeaderStatus::Unspecified
    }
}

#[derive(Debug)]
pub enum ResolvedCell {
    Cell(Box<CellMeta>),
    IssuingDaoInput,
    Null,
}

impl ResolvedCell {
    pub fn cell_meta(&self) -> Option<&CellMeta> {
        match self {
            ResolvedCell::Cell(cell_meta) => Some(cell_meta),
            _ => None,
        }
    }

    pub fn is_issuing_dao_input(&self) -> bool {
        match self {
            ResolvedCell::IssuingDaoInput => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
pub struct ResolvedOutPoint {
    pub cell: ResolvedCell,
    pub header: Option<Box<Header>>,
}

impl ResolvedOutPoint {
    pub fn issuing_dao() -> ResolvedOutPoint {
        ResolvedOutPoint {
            cell: ResolvedCell::IssuingDaoInput,
            header: None,
        }
    }

    pub fn cell_only(cell: CellMeta) -> ResolvedOutPoint {
        ResolvedOutPoint {
            cell: ResolvedCell::Cell(Box::new(cell)),
            header: None,
        }
    }

    pub fn header_only(header: Header) -> ResolvedOutPoint {
        ResolvedOutPoint {
            cell: ResolvedCell::Null,
            header: Some(Box::new(header)),
        }
    }

    pub fn cell_and_header(cell: CellMeta, header: Header) -> ResolvedOutPoint {
        ResolvedOutPoint {
            cell: ResolvedCell::Cell(Box::new(cell)),
            header: Some(Box::new(header)),
        }
    }

    pub fn cell(&self) -> Option<&CellMeta> {
        self.cell.cell_meta()
    }

    pub fn header(&self) -> Option<&Header> {
        self.header.as_ref().map(AsRef::as_ref)
    }
}

/// Transaction with resolved input cells.
#[derive(Debug)]
pub struct ResolvedTransaction<'a> {
    pub transaction: &'a Transaction,
    pub resolved_deps: Vec<ResolvedOutPoint>,
    pub resolved_inputs: Vec<ResolvedOutPoint>,
}

pub trait CellProvider {
    fn cell(&self, out_point: &OutPoint) -> CellStatus;
}

pub struct OverlayCellProvider<'a> {
    overlay: &'a dyn CellProvider,
    cell_provider: &'a dyn CellProvider,
}

impl<'a> OverlayCellProvider<'a> {
    pub fn new(overlay: &'a dyn CellProvider, cell_provider: &'a dyn CellProvider) -> Self {
        Self {
            overlay,
            cell_provider,
        }
    }
}

impl<'a> CellProvider for OverlayCellProvider<'a> {
    fn cell(&self, out_point: &OutPoint) -> CellStatus {
        match self.overlay.cell(out_point) {
            CellStatus::Live(cell_meta) => CellStatus::Live(cell_meta),
            CellStatus::Dead => CellStatus::Dead,
            CellStatus::Unknown => self.cell_provider.cell(out_point),
            CellStatus::Unspecified => CellStatus::Unspecified,
        }
    }
}

pub struct BlockCellProvider<'a> {
    output_indices: FnvHashMap<&'a H256, usize>,
    block: &'a Block,
}

// Transactions are expected to be sorted within a block,
// Transactions have to appear after any transactions upon which they depend
impl<'a> BlockCellProvider<'a> {
    pub fn new(block: &'a Block) -> Result<Self, UnresolvableError> {
        let output_indices: FnvHashMap<&'a H256, usize> = block
            .transactions()
            .iter()
            .enumerate()
            .map(|(idx, tx)| (tx.hash(), idx))
            .collect();

        for (idx, tx) in block.transactions().iter().enumerate() {
            for dep in tx.deps_iter() {
                if let Some(output_idx) = dep
                    .cell
                    .as_ref()
                    .and_then(|cell| output_indices.get(&cell.tx_hash))
                {
                    if *output_idx >= idx {
                        return Err(UnresolvableError::OutOfOrder(dep.clone()));
                    }
                }
            }
            for input_pt in tx.input_pts_iter() {
                if let Some(output_idx) = input_pt
                    .cell
                    .as_ref()
                    .and_then(|cell| output_indices.get(&cell.tx_hash))
                {
                    if *output_idx >= idx {
                        return Err(UnresolvableError::OutOfOrder(input_pt.clone()));
                    }
                }
            }
        }

        Ok(Self {
            output_indices,
            block,
        })
    }
}

impl<'a> CellProvider for BlockCellProvider<'a> {
    fn cell(&self, out_point: &OutPoint) -> CellStatus {
        if out_point.cell.is_none() {
            return CellStatus::Unspecified;
        }
        let out_point = out_point.cell.as_ref().unwrap();

        self.output_indices
            .get(&out_point.tx_hash)
            .and_then(|i| {
                self.block.transactions()[*i]
                    .outputs()
                    .get(out_point.index as usize)
                    .map(|output| {
                        CellStatus::live_cell(CellMeta {
                            cell_output: Some(output.clone()),
                            out_point: out_point.to_owned(),
                            data_hash: None,
                            capacity: output.capacity,
                            block_info: Some(BlockInfo {
                                number: self.block.header().number(),
                                epoch: self.block.header().epoch(),
                            }),
                            cellbase: *i == 0,
                        })
                    })
            })
            .unwrap_or_else(|| CellStatus::Unknown)
    }
}

pub trait HeaderProvider {
    fn header(&self, out_point: &OutPoint) -> HeaderStatus;
}

pub struct OverlayHeaderProvider<'a, O, HP> {
    overlay: &'a O,
    header_provider: &'a HP,
}

impl<'a, O, HP> OverlayHeaderProvider<'a, O, HP> {
    pub fn new(overlay: &'a O, header_provider: &'a HP) -> Self {
        OverlayHeaderProvider {
            overlay,
            header_provider,
        }
    }
}

impl<'a, O, HP> HeaderProvider for OverlayHeaderProvider<'a, O, HP>
where
    O: HeaderProvider,
    HP: HeaderProvider,
{
    fn header(&self, out_point: &OutPoint) -> HeaderStatus {
        match self.overlay.header(out_point) {
            HeaderStatus::Live(h) => HeaderStatus::Live(h),
            HeaderStatus::InclusionFaliure => HeaderStatus::InclusionFaliure,
            HeaderStatus::Unknown => self.header_provider.header(out_point),
            HeaderStatus::Unspecified => HeaderStatus::Unspecified,
        }
    }
}

#[derive(Default)]
pub struct BlockHeadersProvider {
    attached_indices: FnvHashMap<H256, Header>,
    attached_transaction_blocks: FnvHashMap<H256, H256>,
    detached_indices: FnvHashMap<H256, Header>,
}

impl BlockHeadersProvider {
    pub fn push_attached(&mut self, block: &Block) {
        self.attached_indices
            .insert(block.header().hash().clone(), block.header().clone());
        for tx in block.transactions() {
            self.attached_transaction_blocks
                .insert(tx.hash().clone(), block.header().hash().clone());
        }
    }

    pub fn push_detached(&mut self, block: &Block) {
        self.detached_indices
            .insert(block.header().hash().clone(), block.header().clone());
    }

    #[cfg(test)]
    pub fn insert_attached_transaction_block(&mut self, tx_hash: H256, header_hash: H256) {
        self.attached_transaction_blocks
            .insert(tx_hash, header_hash);
    }
}

impl HeaderProvider for BlockHeadersProvider {
    fn header(&self, out_point: &OutPoint) -> HeaderStatus {
        if let Some(block_hash) = &out_point.block_hash {
            if self.detached_indices.contains_key(&block_hash) {
                return HeaderStatus::Unknown;
            }
            match self.attached_indices.get(&block_hash) {
                Some(header) => {
                    if let Some(cell_out_point) = &out_point.cell {
                        self.attached_transaction_blocks
                            .get(&cell_out_point.tx_hash)
                            .map_or(HeaderStatus::InclusionFaliure, |tx_block_hash| {
                                if *tx_block_hash == *block_hash {
                                    HeaderStatus::live_header((*header).clone())
                                } else {
                                    HeaderStatus::InclusionFaliure
                                }
                            })
                    } else {
                        HeaderStatus::live_header((*header).clone())
                    }
                }
                None => HeaderStatus::Unknown,
            }
        } else {
            HeaderStatus::Unspecified
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum UnresolvableError {
    // OutPoint is empty
    Empty,
    // OutPoint is used as input, but a cell is not specified
    UnspecifiedInputCell(OutPoint),
    // OutPoint specifies an invalid header, this could be due to either
    // of the following 2 reasons:
    // 1. Specified header doesn't exist on chain.
    // 2. OutPoint specifies both header and cell, but the specified cell
    // is not included in the specified block header.
    InvalidHeader(OutPoint),
    Dead(OutPoint),
    Unknown(Vec<OutPoint>),
    OutOfOrder(OutPoint),
}

pub fn resolve_transaction<'a, CP: CellProvider, HP: HeaderProvider>(
    transaction: &'a Transaction,
    seen_inputs: &mut FnvHashSet<OutPoint>,
    cell_provider: &CP,
    header_provider: &HP,
) -> Result<ResolvedTransaction<'a>, UnresolvableError> {
    let (mut unknown_out_points, mut resolved_inputs, mut resolved_deps) = (
        Vec::new(),
        Vec::with_capacity(transaction.inputs().len()),
        Vec::with_capacity(transaction.deps().len()),
    );

    // skip resolve input of cellbase
    if !transaction.is_cellbase() {
        for out_point in transaction.input_pts_iter() {
            if out_point.is_issuing_dao() {
                resolved_inputs.push(ResolvedOutPoint::issuing_dao());
                continue;
            }

            let (cell_status, header_status) = if seen_inputs.insert(out_point.to_owned()) {
                (
                    cell_provider.cell(out_point),
                    header_provider.header(out_point),
                )
            } else {
                (CellStatus::Dead, HeaderStatus::Unknown)
            };

            match (cell_status, header_status) {
                (CellStatus::Dead, _) => {
                    return Err(UnresolvableError::Dead(out_point.to_owned()));
                }
                (CellStatus::Unknown, _) => {
                    unknown_out_points.push(out_point.to_owned());
                }
                // Input cell must exist
                (CellStatus::Unspecified, _) => {
                    return Err(UnresolvableError::UnspecifiedInputCell(
                        out_point.to_owned(),
                    ));
                }
                (_, HeaderStatus::Unknown) => {
                    // TODO: should we change transaction pool so transactions
                    // with unknown header can be included as orphans, waiting
                    // for the correct block header to enable it?
                    return Err(UnresolvableError::InvalidHeader(out_point.to_owned()));
                }
                (_, HeaderStatus::InclusionFaliure) => {
                    return Err(UnresolvableError::InvalidHeader(out_point.to_owned()));
                }

                (CellStatus::Live(cell_meta), HeaderStatus::Live(header)) => {
                    resolved_inputs.push(ResolvedOutPoint::cell_and_header(*cell_meta, *header));
                }
                (CellStatus::Live(cell_meta), HeaderStatus::Unspecified) => {
                    resolved_inputs.push(ResolvedOutPoint::cell_only(*cell_meta));
                }
            }
        }
    }

    for out_point in transaction.deps_iter() {
        let cell_status = cell_provider.cell(out_point);
        let header_status = header_provider.header(out_point);

        match (cell_status, header_status) {
            (CellStatus::Dead, _) => {
                return Err(UnresolvableError::Dead(out_point.to_owned()));
            }
            (CellStatus::Unknown, _) => {
                unknown_out_points.push(out_point.to_owned());
            }
            (_, HeaderStatus::Unknown) => {
                // TODO: should we change transaction pool so transactions
                // with unknown header can be included as orphans, waiting
                // for the correct block header to enable it?
                return Err(UnresolvableError::InvalidHeader(out_point.to_owned()));
            }
            (_, HeaderStatus::InclusionFaliure) => {
                return Err(UnresolvableError::InvalidHeader(out_point.to_owned()));
            }
            (CellStatus::Live(_), _) if seen_inputs.contains(&out_point) => {
                return Err(UnresolvableError::Dead(out_point.clone()));
            }
            (CellStatus::Live(cell_meta), HeaderStatus::Live(header)) => {
                resolved_deps.push(ResolvedOutPoint::cell_and_header(*cell_meta, *header));
            }
            (CellStatus::Live(cell_meta), HeaderStatus::Unspecified) => {
                resolved_deps.push(ResolvedOutPoint::cell_only(*cell_meta));
            }
            (CellStatus::Unspecified, HeaderStatus::Live(header)) => {
                resolved_deps.push(ResolvedOutPoint::header_only(*header));
            }
            (CellStatus::Unspecified, HeaderStatus::Unspecified) => {
                return Err(UnresolvableError::Empty);
            }
        }
    }

    if !unknown_out_points.is_empty() {
        Err(UnresolvableError::Unknown(unknown_out_points))
    } else {
        Ok(ResolvedTransaction {
            transaction,
            resolved_inputs,
            resolved_deps,
        })
    }
}

impl<'a> ResolvedTransaction<'a> {
    // cellbase will be resolved with empty input cells, we can use low cost check here:
    pub fn is_cellbase(&self) -> bool {
        self.resolved_inputs.is_empty()
    }

    pub fn inputs_capacity(&self) -> ::occupied_capacity::Result<Capacity> {
        self.resolved_inputs
            .iter()
            .map(|o| {
                o.cell
                    .cell_meta()
                    .map_or_else(Capacity::zero, CellMeta::capacity)
            })
            .try_fold(Capacity::zero(), Capacity::safe_add)
    }
}

#[cfg(test)]
mod tests {
    use super::super::block::{Block, BlockBuilder};
    use super::super::script::Script;
    use super::super::transaction::{CellInput, CellOutPoint, OutPoint, TransactionBuilder};
    use super::*;
    use crate::{capacity_bytes, Bytes, Capacity};
    use numext_fixed_hash::{h256, H256};
    use std::collections::HashMap;

    #[derive(Default)]
    struct CellMemoryDb {
        cells: HashMap<CellOutPoint, Option<CellMeta>>,
    }
    impl CellProvider for CellMemoryDb {
        fn cell(&self, o: &OutPoint) -> CellStatus {
            if o.cell.is_none() {
                return CellStatus::Unspecified;
            }

            match self.cells.get(o.cell.as_ref().unwrap()) {
                Some(&Some(ref cell_meta)) => CellStatus::live_cell(cell_meta.clone()),
                Some(&None) => CellStatus::Dead,
                None => CellStatus::Unknown,
            }
        }
    }

    fn generate_dummy_cell_meta() -> CellMeta {
        let cell_output = CellOutput {
            capacity: capacity_bytes!(2),
            data: Bytes::default(),
            lock: Script::default(),
            type_: None,
        };
        CellMeta {
            block_info: Some(BlockInfo {
                number: 1,
                epoch: 1,
            }),
            capacity: cell_output.capacity,
            data_hash: Some(cell_output.data_hash()),
            cell_output: Some(cell_output),
            out_point: CellOutPoint {
                tx_hash: Default::default(),
                index: 0,
            },
            cellbase: false,
        }
    }

    fn generate_block(txs: Vec<Transaction>) -> Block {
        BlockBuilder::default().transactions(txs).build()
    }

    #[test]
    fn cell_provider_trait_works() {
        let mut db = CellMemoryDb::default();

        let p1 = OutPoint {
            block_hash: None,
            cell: Some(CellOutPoint {
                tx_hash: H256::zero(),
                index: 1,
            }),
        };
        let p2 = OutPoint {
            block_hash: None,
            cell: Some(CellOutPoint {
                tx_hash: H256::zero(),
                index: 2,
            }),
        };
        let p3 = OutPoint {
            block_hash: None,
            cell: Some(CellOutPoint {
                tx_hash: H256::zero(),
                index: 3,
            }),
        };
        let o = generate_dummy_cell_meta();

        db.cells.insert(p1.cell.clone().unwrap(), Some(o.clone()));
        db.cells.insert(p2.cell.clone().unwrap(), None);

        assert_eq!(CellStatus::Live(Box::new(o)), db.cell(&p1));
        assert_eq!(CellStatus::Dead, db.cell(&p2));
        assert_eq!(CellStatus::Unknown, db.cell(&p3));
    }

    #[test]
    fn resolve_transaction_should_resolve_header_only_out_point() {
        let cell_provider = CellMemoryDb::default();
        let mut header_provider = BlockHeadersProvider::default();

        let block = generate_block(vec![]);
        let header_hash = block.header().hash();

        header_provider.push_attached(&block);

        let out_point = OutPoint::new_block_hash(header_hash.clone());
        let transaction = TransactionBuilder::default().dep(out_point).build();

        let mut seen_inputs = FnvHashSet::default();
        let result = resolve_transaction(
            &transaction,
            &mut seen_inputs,
            &cell_provider,
            &header_provider,
        )
        .unwrap();

        assert!(result.resolved_deps[0].cell.cell_meta().is_none());
        assert_eq!(
            result.resolved_deps[0].header,
            Some(Box::new(block.header().clone()))
        );
    }

    #[test]
    fn resolve_transaction_should_reject_input_without_cells() {
        let cell_provider = CellMemoryDb::default();
        let mut header_provider = BlockHeadersProvider::default();

        let block = generate_block(vec![]);
        let header_hash = block.header().hash();

        header_provider.push_attached(&block);

        let out_point = OutPoint::new_block_hash(header_hash.clone());
        let transaction = TransactionBuilder::default()
            .input(CellInput::new(out_point.clone(), 0))
            .build();

        let mut seen_inputs = FnvHashSet::default();
        let result = resolve_transaction(
            &transaction,
            &mut seen_inputs,
            &cell_provider,
            &header_provider,
        );

        assert_eq!(
            result.err(),
            Some(UnresolvableError::UnspecifiedInputCell(out_point))
        );
    }

    #[test]
    fn resolve_transaction_should_resolve_both_header_and_cell() {
        let mut cell_provider = CellMemoryDb::default();
        let mut header_provider = BlockHeadersProvider::default();

        let block = generate_block(vec![]);
        let header_hash = block.header().hash();
        let out_point = OutPoint::new(header_hash.clone(), h256!("0x2"), 3);

        cell_provider.cells.insert(
            out_point.cell.clone().unwrap(),
            Some(generate_dummy_cell_meta()),
        );
        header_provider.push_attached(&block);
        header_provider.insert_attached_transaction_block(
            out_point.cell.clone().unwrap().tx_hash,
            header_hash.clone(),
        );

        let transaction = TransactionBuilder::default().dep(out_point).build();

        let mut seen_inputs = FnvHashSet::default();
        let result = resolve_transaction(
            &transaction,
            &mut seen_inputs,
            &cell_provider,
            &header_provider,
        )
        .unwrap();

        assert!(result.resolved_deps[0].cell.cell_meta().is_some());
        assert_eq!(
            result.resolved_deps[0].header,
            Some(Box::new(block.header().clone()))
        );
    }

    #[test]
    fn resolve_transaction_should_test_header_includes_cell() {
        let mut cell_provider = CellMemoryDb::default();
        let mut header_provider = BlockHeadersProvider::default();

        let block = generate_block(vec![]);
        let header_hash = block.header().hash();
        let out_point = OutPoint::new(header_hash.clone(), h256!("0x2"), 3);

        cell_provider.cells.insert(
            out_point.cell.clone().unwrap(),
            Some(generate_dummy_cell_meta()),
        );
        header_provider.push_attached(&block);

        let transaction = TransactionBuilder::default().dep(out_point.clone()).build();

        let mut seen_inputs = FnvHashSet::default();
        let result = resolve_transaction(
            &transaction,
            &mut seen_inputs,
            &cell_provider,
            &header_provider,
        );

        assert_eq!(
            result.err(),
            Some(UnresolvableError::InvalidHeader(out_point))
        );
    }

    #[test]
    fn resolve_transaction_should_reject_empty_out_point() {
        let mut cell_provider = CellMemoryDb::default();
        let mut header_provider = BlockHeadersProvider::default();

        let block = generate_block(vec![]);
        let header_hash = block.header().hash();
        let out_point = OutPoint::new(header_hash.clone(), h256!("0x2"), 3);

        cell_provider.cells.insert(
            out_point.cell.clone().unwrap(),
            Some(generate_dummy_cell_meta()),
        );
        header_provider.push_attached(&block);
        header_provider.insert_attached_transaction_block(
            out_point.cell.clone().unwrap().tx_hash,
            header_hash.clone(),
        );

        let transaction = TransactionBuilder::default()
            .dep(OutPoint::default())
            .build();

        let mut seen_inputs = FnvHashSet::default();
        let result = resolve_transaction(
            &transaction,
            &mut seen_inputs,
            &cell_provider,
            &header_provider,
        );

        assert_eq!(result.err(), Some(UnresolvableError::Empty));
    }

    #[test]
    fn resolve_transaction_should_reject_incorrect_order_txs() {
        let out_point = OutPoint::new_cell(h256!("0x2"), 3);

        let tx1 = TransactionBuilder::default()
            .input(CellInput::new(out_point.clone(), 0))
            .output(CellOutput::new(
                capacity_bytes!(2),
                Bytes::default(),
                Script::default(),
                None,
            ))
            .build();

        let tx2 = TransactionBuilder::default()
            .input(CellInput::new(
                OutPoint::new_cell(tx1.hash().to_owned(), 0),
                0,
            ))
            .build();

        let tx3 = TransactionBuilder::default()
            .dep(OutPoint::new_cell(tx1.hash().to_owned(), 0))
            .build();

        // tx1 <- tx2
        // ok
        {
            let block = generate_block(vec![tx1.clone(), tx2.clone()]);
            let provider = BlockCellProvider::new(&block);
            assert!(provider.is_ok());
        }

        // tx1 -> tx2
        // resolve err
        {
            let block = generate_block(vec![tx2.clone(), tx1.clone()]);
            let provider = BlockCellProvider::new(&block);

            assert_eq!(
                provider.err(),
                Some(UnresolvableError::OutOfOrder(OutPoint::new_cell(
                    tx1.hash().to_owned(),
                    0
                )))
            );
        }

        // tx1 <- tx3
        // ok
        {
            let block = generate_block(vec![tx1.clone(), tx3.clone()]);
            let provider = BlockCellProvider::new(&block);

            assert!(provider.is_ok());
        }

        // tx1 -> tx3
        // resolve err
        {
            let block = generate_block(vec![tx3.clone(), tx1.clone()]);
            let provider = BlockCellProvider::new(&block);

            assert_eq!(
                provider.err(),
                Some(UnresolvableError::OutOfOrder(OutPoint::new_cell(
                    tx1.hash().to_owned(),
                    0
                )))
            );
        }
    }

    #[test]
    fn resolve_transaction_should_reject_dep_cell_consumed_by_previous_input() {
        let mut cell_provider = CellMemoryDb::default();
        let header_provider = BlockHeadersProvider::default();

        let out_point = OutPoint::new_cell(h256!("0x2"), 3);

        cell_provider.cells.insert(
            out_point.cell.clone().unwrap(),
            Some(generate_dummy_cell_meta()),
        );

        // dep's outpoint consumed by input
        {
            let tx = TransactionBuilder::default()
                .input(CellInput::new(out_point.clone(), 0))
                .dep(out_point.clone())
                .build();

            let mut seen_inputs = FnvHashSet::default();
            let result =
                resolve_transaction(&tx, &mut seen_inputs, &cell_provider, &header_provider);

            assert_eq!(
                result.err(),
                Some(UnresolvableError::Dead(out_point.clone()))
            );
        }

        // tx1 dep
        // tx2 input consumed
        // ok
        {
            let tx1 = TransactionBuilder::default().dep(out_point.clone()).build();
            let tx2 = TransactionBuilder::default()
                .input(CellInput::new(out_point.clone(), 0))
                .build();

            let mut seen_inputs = FnvHashSet::default();
            let result1 =
                resolve_transaction(&tx1, &mut seen_inputs, &cell_provider, &header_provider);
            assert!(result1.is_ok());

            let result2 =
                resolve_transaction(&tx2, &mut seen_inputs, &cell_provider, &header_provider);
            assert!(result2.is_ok());
        }

        // tx1 input consumed
        // tx2 dep
        // tx2 resolve err
        {
            let tx1 = TransactionBuilder::default()
                .input(CellInput::new(out_point.clone(), 0))
                .build();

            let tx2 = TransactionBuilder::default().dep(out_point.clone()).build();

            let mut seen_inputs = FnvHashSet::default();
            let result1 =
                resolve_transaction(&tx1, &mut seen_inputs, &cell_provider, &header_provider);

            assert!(result1.is_ok());

            let result2 =
                resolve_transaction(&tx2, &mut seen_inputs, &cell_provider, &header_provider);

            assert_eq!(
                result2.err(),
                Some(UnresolvableError::Dead(out_point.clone()))
            );
        }
    }
}
