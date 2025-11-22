use crate::semantic::types::InferredType;

#[derive(Debug, Clone, Copy)]
pub enum BuiltinIntrinsic {
    Say,
    Ask,
    Read,
    Write,
}

#[derive(Debug, Clone)]
pub struct BuiltinFunc {
    pub name: &'static str,
    pub arity: usize,
    pub variadic: bool,
    pub returns: InferredType,
    pub intrinsic: Option<BuiltinIntrinsic>,
}

#[derive(Debug, Clone)]
pub struct BuiltinMethod {
    pub name: &'static str,
    pub arity: usize,
    pub variadic: bool,
    pub returns: InferredType,
}

lazy_static::lazy_static! {
    pub static ref BUILTIN_FUNCS: std::collections::HashMap<&'static str, BuiltinFunc> = {
        use InferredType::*;
        use BuiltinIntrinsic::*;
        let mut m = std::collections::HashMap::new();
        m.insert("say",   BuiltinFunc { name:"say",   arity:1, variadic:false, returns: Unit,    intrinsic: Some(Say) });
        m.insert("ask",   BuiltinFunc { name:"ask",   arity:1, variadic:false, returns: Dynamic, intrinsic: Some(Ask) });
        m.insert("read",  BuiltinFunc { name:"read",  arity:1, variadic:false, returns: Str,     intrinsic: Some(Read) });
        m.insert("write", BuiltinFunc { name:"write", arity:2, variadic:false, returns: Unit,    intrinsic: Some(Write) });
        m
    };
}

lazy_static::lazy_static! {
    pub static ref BUILTIN_METHODS: std::collections::HashMap<&'static str, BuiltinMethod> = {
        use InferredType::*;
        let mut m = std::collections::HashMap::new();
        m.insert("length", BuiltinMethod { name:"length", arity:0, variadic:false, returns: Int });
        m
    };
}