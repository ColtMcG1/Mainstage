use crate::ir::slot::Slot;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

pub type ScopeRef = Rc<RefCell<Scope>>;

pub struct Scope {
    pub name: String,
    pub parent: Option<Weak<RefCell<Scope>>>,
    pub symbols: HashMap<String, Slot>,  // locals in this scope
    pub captured: HashMap<String, Slot>, // names captured from parents -> Captured slots
    next_temp: usize,
    next_local: usize,
    next_captured: usize,
}

impl Scope {
    pub fn new_root(name: impl Into<String>) -> ScopeRef {
        Rc::new(RefCell::new(Self {
            name: name.into(),
            parent: None,
            symbols: HashMap::new(),
            captured: HashMap::new(),
            next_temp: 0,
            next_local: 0,
            next_captured: 0,
        }))
    }

    pub fn child(parent: &ScopeRef, name: impl Into<String>) -> ScopeRef {
        Rc::new(RefCell::new(Self {
            name: name.into(),
            parent: Some(Rc::downgrade(parent)),
            symbols: HashMap::new(),
            captured: HashMap::new(),
            next_temp: 0,
            next_local: 0,
            next_captured: 0,
        }))
    }

    pub fn define_local(&mut self, name: impl Into<String>) -> Slot {
        let s = name.into();
        let slot = Slot::Local(self.next_local);
        self.next_local += 1;
        self.symbols.insert(s, slot);
        slot
    }

    pub fn allocate_temp(&mut self) -> Slot {
        let slot = Slot::Temp(self.next_temp);
        self.next_temp += 1;
        slot
    }

    pub fn resolve(&mut self, name: &str) -> Option<Slot> {
        if let Some(&slot) = self.symbols.get(name) {
            return Some(slot);
        } else if let Some(&slot) = self.captured.get(name) {
            return Some(slot);
        } else {
            // climb parent chain
            let parent_rc = self.parent.as_ref()?.upgrade()?;
            if let Some(_) = parent_rc.borrow_mut().resolve(name) {
                // capture locally with a Captured slot index
                let captured_slot = Slot::Captured(self.next_captured);
                self.next_captured += 1;
                self.captured.insert(name.to_string(), captured_slot);
                Some(captured_slot)
            } else {
                None
            }
        }
    }
}
