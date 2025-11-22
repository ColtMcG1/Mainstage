use std::collections::HashMap;
use crate::ir::{op::Op, slot::Slot};
use crate::ir::optimization::pass::Pass;

/// Copy Propagation
/// - Tracks simple copies (LoadLocal/LoadGlobal producing a temp) and replaces later uses
///   of that temp with the original source slot.
/// - Conservative: does not change semantics for mutated slots.
pub struct CopyPropagation;
impl CopyPropagation { pub fn new() -> Self { Self } }

impl Pass for CopyPropagation {
    fn name(&self) -> &'static str { "copy_prop" }

    fn run(&mut self, ops: &mut Vec<Op>) -> bool {
        // map: temp slot -> original slot (the slot it was copied from)
        let mut copies: HashMap<Slot, Slot> = HashMap::new();

        // detect mutated slots to avoid unsound propagation
        let mut mutated = std::collections::HashSet::new();
        for op in ops.iter() {
            match op {
                Op::StoreLocal { target, .. } => { mutated.insert(*target); }
                Op::StoreGlobal { source, .. } => { mutated.insert(*source); }
                Op::Inc { target } | Op::Dec { target } => { mutated.insert(*target); }
                Op::ISet { target, .. } => { mutated.insert(*target); }
                _ => {}
            }
        }

        // first pass: collect copy relationships
        for op in ops.iter() {
            match op {
                Op::LoadLocal { target, local: source } => {
                    // target is a copy of local
                    if !mutated.contains(source) {
                        copies.insert(*target, *source);
                    }
                }
                Op::LoadGlobal { target, .. } => {
                    if !mutated.contains(target) {
                        copies.insert(*target, *target);
                    }
                }
                Op::LoadConst { target, .. } => {
                    // load const is a copy from no-slot; drop mapping (a const)
                    copies.remove(target);
                }
                _ => {
                    // ops that define a slot should remove any copy mapping for that target
                    if let Some(def) = op.defines_slot() {
                        copies.remove(&def);
                    }
                }
            }
        }

        if copies.is_empty() { return false; }

        // utility: resolve final representative (chain replace)
        let resolve = |mut s: Slot| {
            while let Some(next) = copies.get(&s) {
                // stop if next is same or maps to itself or mutated
                if *next == s { break; }
                s = *next;
            }
            s
        };

        // apply replacements to all ops' used slots
        let mut changed = false;
        for op in ops.iter_mut() {
            // skip ops that would be unsafe if they use mutated slots as destination etc.
            op.map_used_slots(|slot| {
                let rep = resolve(slot);
                if rep != slot {
                    changed = true;
                    rep
                } else {
                    slot
                }
            });
        }

        changed
    }
}