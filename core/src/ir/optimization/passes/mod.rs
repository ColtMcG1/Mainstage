mod constant_folding;
mod dce;

pub use constant_folding::ConstantFolding;
pub use dce::DeadCodeElim;