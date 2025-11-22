use super::pass::Pass;
use super::passes::{
    CommonSubexpressionElimination, ConstantFolding, ConstantPropagation, CopyPropagation,
    DeadCodeElim, SparseConditionalConstantProp,
};
use crate::ir::op::Op;

pub struct Pipeline {
    passes: Vec<Box<dyn Pass>>,
    pub max_iterations: usize,
}

impl Pipeline {
    pub fn new() -> Self {
        Self { passes: Vec::new(), max_iterations: 5 }
    }

    pub fn with_default_passes() -> Self {
        let mut p = Self::new();
        // safe ordering:
        // 1) constant folding (local algebraic folding)
        // 2) SCCP (discover conditional/global constants)
        // 3) constant propagation
        // 4) copy propagation
        // 5) CSE (deduplicate pure expressions using canonical temps)
        // 6) DCE (remove now-unused producers)
        p.add_pass(Box::new(ConstantFolding::new()));
        p.add_pass(Box::new(SparseConditionalConstantProp::new()));
        p.add_pass(Box::new(ConstantPropagation::new()));
        p.add_pass(Box::new(CopyPropagation::new()));
        p.add_pass(Box::new(CommonSubexpressionElimination::new()));
        p.add_pass(Box::new(DeadCodeElim::new()));
        p
    }

    pub fn add_pass(&mut self, pass: Box<dyn Pass>) { self.passes.push(pass); }

    pub fn run(&mut self, ops: &mut Vec<Op>) {
        // iterate the whole pass sequence until no pass makes progress, or until max_iterations
        for iter in 0..self.max_iterations {
            let mut any_changed = false;
            for pass in self.passes.iter_mut() {
                let changed = pass.run(ops);
                if changed {
                    any_changed = true;
                }
            }
            if !any_changed {
                // fixed point reached
                break;
            } else {
                eprintln!("opt: iteration {} end, ops = {}", iter, ops.len());
            }
        }
    }
}

pub fn optimize_ops_default(ops: &mut Vec<Op>) {
    let mut pipe = Pipeline::with_default_passes();
    pipe.run(ops);
}
