use crate::model::{Block, CodeUnit, Node};

pub fn cyclomatic(u: &CodeUnit) -> u32 {
    1 + count_branches(&u.body)
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
