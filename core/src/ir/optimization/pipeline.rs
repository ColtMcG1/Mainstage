use crate::ir::op::Op;
use super::pass::Pass;
use super::passes::{ConstantFolding, DeadCodeElim};

pub struct Pipeline { passes: Vec<Box<dyn Pass>> }

impl Pipeline {
    pub fn new() -> Self { Self { passes: Vec::new() } }

    pub fn with_default_passes() -> Self {
        let mut p = Self::new();
        p.add_pass(Box::new(ConstantFolding::new()));
        p.add_pass(Box::new(DeadCodeElim::new()));
        p
    }

    pub fn add_pass(&mut self, pass: Box<dyn Pass>) { self.passes.push(pass); }

    pub fn run(&mut self, ops: &mut Vec<Op>) {
        // iterate to fixpoint (max 5)
        for _ in 0..5 {
            let mut any = false;
            for pass in self.passes.iter_mut() {
                if pass.run(ops) { any = true; }
            }
            if !any { break; }
        }
    }
}

pub fn optimize_ops_default(ops: &mut Vec<Op>) {
    let mut pipe = Pipeline::with_default_passes();
    pipe.run(ops);
}