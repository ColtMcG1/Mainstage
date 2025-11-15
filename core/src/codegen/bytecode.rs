use std::collections::HashMap;
use crate::codegen::ir::{IRConst, IRFunction, IROpKind, ModuleIR};
use crate::runtime::opcode::Op;

// Preserve function name for VM dispatch.
#[derive(Debug)]
pub struct BytecodeFunction {
    pub name: String,
    pub code: Vec<u8>,
    pub label_map: HashMap<u32, usize>,
    pub patch_sites: Vec<(usize, u32)>,
}

#[derive(Debug)]
pub struct BytecodeModule {
    pub const_pool: Vec<IRConst>,
    pub functions: Vec<BytecodeFunction>,
    pub name_to_global: HashMap<String, usize>,
}

fn push_u8(buf: &mut Vec<u8>, v: u8) { buf.push(v); }
fn push_u32(buf: &mut Vec<u8>, v: u32) { buf.extend_from_slice(&v.to_le_bytes()); }

fn encode_func(f: &IRFunction) -> BytecodeFunction {
    let mut bf = BytecodeFunction { name: f.name.clone(), code: Vec::new(), label_map: HashMap::new(), patch_sites: Vec::new() };

    // First pass: assign labels to offsets
    for block in &f.blocks {
        bf.label_map.insert(block.label, bf.code.len());
        // Emit ops
        for op in &block.ops {
            match op.kind {
                IROpKind::LoadConst(idx) => {
                    push_u8(&mut bf.code, Op::LoadConst as u8);
                    push_u32(&mut bf.code, idx);
                }
                IROpKind::LoadVar(id) => {
                    push_u8(&mut bf.code, Op::LoadVar as u8);
                    push_u32(&mut bf.code, id);
                }
                IROpKind::StoreVar(id) => {
                    push_u8(&mut bf.code, Op::StoreVar as u8);
                    push_u32(&mut bf.code, id);
                }
                IROpKind::StoreGlobal(id) => {
                    push_u8(&mut bf.code, Op::StoreGlobal as u8);
                    push_u32(&mut bf.code, id);
                }
                IROpKind::Add => push_u8(&mut bf.code, Op::Add as u8),
                IROpKind::Sub => push_u8(&mut bf.code, Op::Sub as u8),
                IROpKind::Mul => push_u8(&mut bf.code, Op::Mul as u8),
                IROpKind::Div => push_u8(&mut bf.code, Op::Div as u8),
                IROpKind::Concat => push_u8(&mut bf.code, Op::Concat as u8),
                IROpKind::Jump(label) => {
                    push_u8(&mut bf.code, Op::Jump as u8);
                    let pos = bf.code.len();
                    push_u32(&mut bf.code, 0);
                    bf.patch_sites.push((pos, label));
                }
                IROpKind::JumpIfFalse(label) => {
                    push_u8(&mut bf.code, Op::JumpIfFalse as u8);
                    let pos = bf.code.len();
                    push_u32(&mut bf.code, 0);
                    bf.patch_sites.push((pos, label));
                }
                IROpKind::Call(fid, argc) => {
                    push_u8(&mut bf.code, Op::Call as u8);
                    push_u32(&mut bf.code, fid);
                    push_u8(&mut bf.code, argc);
                }
                IROpKind::Return => push_u8(&mut bf.code, Op::Return as u8),
                IROpKind::Say => push_u8(&mut bf.code, Op::Say as u8),
                IROpKind::Ask(argc) => {
                    push_u8(&mut bf.code, Op::Ask as u8);
                    push_u8(&mut bf.code, argc);
                }
                IROpKind::Read => push_u8(&mut bf.code, Op::Read as u8),
                IROpKind::Write => push_u8(&mut bf.code, Op::Write as u8),
                IROpKind::LoadMemberDyn(idx) => {
                    push_u8(&mut bf.code, Op::LoadMemberDyn as u8);
                    push_u32(&mut bf.code, idx);
                }
                IROpKind::Index => {
                    push_u8(&mut bf.code, Op::Index as u8);
                }
                IROpKind::Pop => {
                    push_u8(&mut bf.code, Op::Pop as u8);
                }
                IROpKind::LoadRefMember(idx) => {
                    push_u8(&mut bf.code, Op::LoadRefMember as u8);
                    push_u32(&mut bf.code, idx);
                }
                IROpKind::NoOp => push_u8(&mut bf.code, Op::NoOp as u8),
            }
        }
    }

    // Patch jumps
    for (pos, target) in bf.patch_sites.clone() {
        if let Some(&dest) = bf.label_map.get(&target) {
            let bytes = (dest as u32).to_le_bytes();
            bf.code[pos..pos + 4].copy_from_slice(&bytes);
        }
    }

    bf
}

pub fn emit_bytecode(module: &ModuleIR) -> BytecodeModule {
    let mut out = BytecodeModule {
        const_pool: module.const_pool.clone(),
        functions: Vec::new(),
        name_to_global: module.globals.iter().enumerate()
            .map(|(i, name)| (name.clone(), i))
            .collect(),
    };

    for f in &module.functions {
        out.functions.push(encode_func(f));
    }

    out
}