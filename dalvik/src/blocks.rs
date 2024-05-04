//! Basic block lifting

use std::collections::{BTreeMap, BTreeSet};

use crate::{ControlFlow, Instruction};

/// A simple basic block of sequential instructions.
///
/// Unless an instruction raises an exception, every instruction except the last
/// in `instructions` is expected to fall through to the next after it executes.
///
/// The final instruction may fall through to the next addressed instruction
/// (outside of this basic block), or it may have multiple jump locations based
/// on a conditional, or it may even return from the method, terminating local
/// control flow. The `next` Vec stores this information.
#[derive(Debug)]
pub struct BasicBlock {
    /// Instructions contained in this basic block
    pub instructions: Vec<Instruction>,
    /// Next branch targets from the last instruction of this block
    pub next: NextBranch,
}

/// Possible branch targets finalizing a basic block
#[derive(Debug)]
pub enum NextBranch {
    /// Termination (e.g. return or throw)
    None,
    /// Unconditional jump
    Goto(usize),
    /// Conditional jump
    Cond {
        /// Branch here if condition is true
        t: usize,
        /// Branch here if condition is false
        f: usize,
    },
}

impl NextBranch {
    /// Iterator over possible branch targets
    pub fn iter(&self) -> impl Iterator<Item = usize> {
        match self {
            NextBranch::None => [None, None].into_iter().flatten(),
            NextBranch::Goto(t) => [Some(*t), None].into_iter().flatten(),
            NextBranch::Cond { t, f } => [Some(*t), Some(*f)].into_iter().flatten(),
        }
    }
}

/// Parse a method's dalvik bytecode into [`BasicBlock`]s keyed by their
/// bytecode start offset/address. `entries` should be all known entrypoints
/// within the method, for example the offsets of the exception handling catch
/// blocks, parsed from the relevant [dex table].
///
/// [dex]: https://source.android.com/docs/core/runtime/dex-format#type-item

// TODO: implement block splitting to reduce total blocks returned
pub fn basic_blocks(bytecode: &[u16], entries: &[usize]) -> BTreeMap<usize, BasicBlock> {
    let mut bbs = BTreeMap::new();
    let mut search_next = BTreeSet::from([0]);
    for e in entries {
        search_next.insert(*e);
    }

    while let Some(start_addr) = search_next.pop_first() {
        let bb = decode_bb(bytecode, start_addr, &search_next);
        for next in bb.next.iter() {
            if !bbs.contains_key(&next) {
                search_next.insert(next);
            }
        }
        bbs.insert(start_addr, bb);
    }

    bbs
}

// decode a single basic block starting at entry_point, and stopping before any other known entry_points
fn decode_bb(bytecode: &[u16], entry_point: usize, avoid: &BTreeSet<usize>) -> BasicBlock {
    let mut instructions = Vec::new();
    let next;

    let mut cursor = entry_point;

    loop {
        let inst = crate::decode::decode_one(&mut &bytecode[cursor..]).unwrap();
        let cf = inst.control_flow();
        let len = inst.len();
        instructions.push(inst);

        next = match cf {
            ControlFlow::FallThrough => {
                cursor += len;
                if !avoid.contains(&cursor) {
                    continue;
                }
                NextBranch::Goto(cursor)
            }
            ControlFlow::GoTo(t) => NextBranch::Goto((cursor as i32 + t) as usize),
            ControlFlow::Branch(t) => NextBranch::Cond {
                t: (cursor as i32 + t as i32) as usize,
                f: cursor + len,
            },

            ControlFlow::Terminate => NextBranch::None,
        };

        break;
    }

    BasicBlock { instructions, next }
}
