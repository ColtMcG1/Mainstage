mod constant_folding;
mod const_prop;
mod copy_prop;
mod dce;

pub use constant_folding::ConstantFolding;
pub use const_prop::ConstantPropagation;
pub use copy_prop::CopyPropagation;
pub use dce::DeadCodeElim;