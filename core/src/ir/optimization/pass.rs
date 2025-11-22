use crate::ir::op::Op;

pub trait Pass {
    fn name(&self) -> &'static str;
    fn run(&mut self, ops: &mut Vec<Op>) -> bool; // return true if changed
}