use crate::model::{Block, CodeUnit, Node, Metric};

pub struct CyclomaticComplexity;

impl Metric for CyclomaticComplexity {
    fn name(&self) -> &'static str {
        "Cyclomatic Complexity"
    }

    fn calculate(&self, unit: &CodeUnit) -> u32 {
        1 + count_branches(&unit.body)
    }
}

fn count_branches(block: &Block) -> u32 {
    block
        .children
        .iter()
        .map(|n| match n {
            Node::Branch(_) => 1,
            Node::NestedBlock(b) => count_branches(b),
            _ => 0,
        })
        .sum()
}
