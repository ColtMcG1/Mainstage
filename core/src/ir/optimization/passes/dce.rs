use std::collections::HashMap;
use crate::ir::op::Op;
use crate::ir::slot::Slot;
use crate::ir::optimization::pass::Pass;

pub struct DeadCodeElim;
impl DeadCodeElim { pub fn new() -> Self { Self } }

impl Pass for DeadCodeElim {
    fn name(&self) -> &'static str { "dce" }
    fn run(&mut self, ops: &mut Vec<Op>) -> bool {
        // usage count
        let mut uses: HashMap<Slot, usize> = HashMap::new();
        for op in ops.iter() {
            op.each_used_slot(|s| { *uses.entry(s).or_insert(0) += 1; });
        }
        let mut out = Vec::with_capacity(ops.len());
        let mut changed = false;
        for op in ops.drain(..) {
            let removable = op.is_pure() && op.defines_slot()
                .map(|s| uses.get(&s).copied().unwrap_or(0) == 0)
                .unwrap_or(false);
            if removable {
                changed = true;
            } else {
                out.push(op);
            }
        }
        *ops = out;
        changed
    }
}