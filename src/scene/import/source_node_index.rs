use std::collections::HashMap;

use super::types::ImportedNode;

pub(super) struct SourceNodeIndex<'a> {
    records: &'a [ImportedNode],
    offsets: HashMap<usize, usize>,
}

impl<'a> SourceNodeIndex<'a> {
    pub(super) fn new(records: &'a [ImportedNode]) -> Self {
        Self {
            records,
            offsets: records
                .iter()
                .enumerate()
                .map(|(offset, record)| (record.source_index, offset))
                .collect(),
        }
    }

    pub(super) fn get(&self, source_index: usize) -> Option<&ImportedNode> {
        self.offsets
            .get(&source_index)
            .and_then(|offset| self.records.get(*offset))
    }
}
