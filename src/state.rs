use std::hash::Hash;
use std::mem::ManuallyDrop;
use std::sync::RwLock;
use std::thread::LocalKey;
use std::{
    cell::RefCell,
    collections::hash_map::Entry,
    ops::{Deref, DerefMut},
};

use ahash::{HashMap, HashMapExt};
use append_only_vec::AppendOnlyVec;
use once_cell::sync::Lazy;
use smartstring::alias::String;

use crate::{
    coefficient::Coefficient,
    domains::finite_field::{FiniteField, FiniteFieldCore},
    representations::{Atom, Symbol},
    LicenseManager, LICENSE_MANAGER,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FiniteFieldIndex(pub(crate) usize);

#[derive(Clone, Copy, PartialEq)]
pub enum FunctionAttribute {
    Symmetric,
    Antisymmetric,
    Linear,
}

static STATE: Lazy<RwLock<State>> = Lazy::new(|| RwLock::new(State::new()));
static ID_TO_STR: AppendOnlyVec<String> = AppendOnlyVec::<String>::new();
static FINITE_FIELDS: AppendOnlyVec<FiniteField<u64>> = AppendOnlyVec::<FiniteField<u64>>::new();

thread_local!(
    /// A thread-local workspace, that stores recyclable atoms. By making it const and
    /// `ManuallyDrop`, the fastest implementation is chosen for the current platform.
    /// In principle this leaks memory, but Symbolica only uses thread pools that live as
    /// long as the main thread, so this is no issue.
    static WORKSPACE: ManuallyDrop<Workspace> = const { ManuallyDrop::new(Workspace::new()) }
);

/// A global state, that stores mappings from variable and function names to ids.
pub struct State {
    str_to_id: HashMap<String, Symbol>,
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl State {
    pub const ARG: Symbol = Symbol::init_fn(0, 0, false, false, false);
    pub const COEFF: Symbol = Symbol::init_fn(1, 0, false, false, false);
    pub const EXP: Symbol = Symbol::init_fn(2, 0, false, false, false);
    pub const LOG: Symbol = Symbol::init_fn(3, 0, false, false, false);
    pub const SIN: Symbol = Symbol::init_fn(4, 0, false, false, false);
    pub const COS: Symbol = Symbol::init_fn(5, 0, false, false, false);
    pub const SQRT: Symbol = Symbol::init_fn(6, 0, false, false, false);
    pub const DERIVATIVE: Symbol = Symbol::init_fn(7, 0, false, false, false);
    pub const E: Symbol = Symbol::init_var(8, 0);
    pub const I: Symbol = Symbol::init_var(9, 0);
    pub const PI: Symbol = Symbol::init_var(10, 0);

    pub const BUILTIN_VAR_LIST: [&'static str; 11] = [
        "arg", "coeff", "exp", "log", "sin", "cos", "sqrt", "der", "𝑒", "𝑖", "𝜋",
    ];

    fn new() -> State {
        LICENSE_MANAGER.get_or_init(LicenseManager::new).check();

        let mut state = State {
            str_to_id: HashMap::new(),
        };

        for x in Self::BUILTIN_VAR_LIST {
            state.get_or_insert_var_impl(x);
        }

        state
    }

    /// Get the global state.
    #[inline]
    pub(crate) fn get_global_state() -> &'static RwLock<State> {
        &STATE
    }

    /// Iterate over all defined symbols.
    pub fn symbol_iter<'a>() -> impl Iterator<Item = &'a str> {
        ID_TO_STR.iter().map(|s| s.as_str())
    }

    /// Returns `true` iff this identifier is defined by Symbolica.
    pub fn is_builtin(id: Symbol) -> bool {
        id.get_id() < Self::BUILTIN_VAR_LIST.len() as u32
    }

    /// Get the id for a certain name if the name is already registered,
    /// else register it and return a new id.
    pub fn get_or_insert_var<S: AsRef<str>>(name: S) -> Symbol {
        STATE.write().unwrap().get_or_insert_var_impl(name.as_ref())
    }

    pub(crate) fn get_or_insert_var_impl(&mut self, name: &str) -> Symbol {
        match self.str_to_id.entry(name.into()) {
            Entry::Occupied(o) => *o.get(),
            Entry::Vacant(v) => {
                if ID_TO_STR.len() == u32::MAX as usize - 1 {
                    panic!("Too many variables defined");
                }

                let mut wildcard_level = 0;
                for x in name.chars().rev() {
                    if x != '_' {
                        break;
                    }
                    wildcard_level += 1;
                }

                // there is no synchronization issue since only one thread can insert at a time
                // as the state itself is behind a mutex
                let new_index = ID_TO_STR.push(name.into());

                let new_id = Symbol::init_var(new_index as u32, wildcard_level);
                v.insert(new_id);
                new_id
            }
        }
    }

    /// Get the id of a certain function name if the name is already registered,
    /// else register it and return a new id.
    ///
    /// Providing an attribute `None` means that the attributes will be fetched from
    /// the state if the function exists, or the attribute list will be empty if not.
    pub fn get_or_insert_fn<S: AsRef<str>>(
        name: S,
        attributes: Option<Vec<FunctionAttribute>>,
    ) -> Result<Symbol, String> {
        STATE
            .write()
            .unwrap()
            .get_or_insert_fn_impl(name.as_ref(), attributes)
    }

    pub(crate) fn get_or_insert_fn_impl(
        &mut self,
        name: &str,
        attributes: Option<Vec<FunctionAttribute>>,
    ) -> Result<Symbol, String> {
        match self.str_to_id.entry(name.into()) {
            Entry::Occupied(o) => {
                let r = *o.get();
                if let Some(attributes) = attributes {
                    let new_id = Symbol::init_fn(
                        r.get_id(),
                        r.get_wildcard_level(),
                        attributes.contains(&FunctionAttribute::Symmetric),
                        attributes.contains(&FunctionAttribute::Antisymmetric),
                        attributes.contains(&FunctionAttribute::Linear),
                    );

                    if r == new_id {
                        Ok(r)
                    } else {
                        Err(format!("Function {} redefined with new attributes", name).into())
                    }
                } else {
                    Ok(r)
                }
            }
            Entry::Vacant(v) => {
                if ID_TO_STR.len() == u32::MAX as usize - 1 {
                    panic!("Too many variables defined");
                }

                // there is no synchronization issue since only one thread can insert at a time
                // as the state itself is behind a mutex
                let new_index = ID_TO_STR.push(name.into());

                let mut wildcard_level = 0;
                for x in name.chars().rev() {
                    if x != '_' {
                        break;
                    }
                    wildcard_level += 1;
                }

                let new_id = if let Some(attributes) = attributes {
                    Symbol::init_fn(
                        new_index as u32,
                        wildcard_level,
                        attributes.contains(&FunctionAttribute::Symmetric),
                        attributes.contains(&FunctionAttribute::Antisymmetric),
                        attributes.contains(&FunctionAttribute::Linear),
                    )
                } else {
                    Symbol::init_fn(new_index as u32, wildcard_level, false, false, false)
                };

                v.insert(new_id);

                Ok(new_id)
            }
        }
    }

    /// Get the name for a given symbol.
    pub fn get_name<'a>(id: Symbol) -> &'a String {
        &ID_TO_STR[id.get_id() as usize]
    }

    pub fn get_finite_field<'a>(fi: FiniteFieldIndex) -> &'a FiniteField<u64> {
        &FINITE_FIELDS[fi.0]
    }

    pub fn get_or_insert_finite_field(f: FiniteField<u64>) -> FiniteFieldIndex {
        STATE.write().unwrap().get_or_insert_finite_field_impl(f)
    }

    pub(crate) fn get_or_insert_finite_field_impl(
        &mut self,
        f: FiniteField<u64>,
    ) -> FiniteFieldIndex {
        for (i, f2) in FINITE_FIELDS.iter().enumerate() {
            if f.get_prime() == f2.get_prime() {
                return FiniteFieldIndex(i);
            }
        }

        let index = FINITE_FIELDS.push(f);
        FiniteFieldIndex(index)
    }
}

/// A workspace that stores recyclable atoms. Upon dropping, the atoms automatically returned to a
/// thread-local workspace (which may be a different one than the one it was created by).
pub struct Workspace {
    atom_buffer: RefCell<Vec<Atom>>,
}

impl Workspace {
    const ATOM_BUFFER_MAX: usize = 25;

    /// Create a new workspace.
    const fn new() -> Self {
        Workspace {
            atom_buffer: RefCell::new(Vec::new()),
        }
    }

    /// Get a thread-local workspace.
    #[inline]
    pub const fn get_local() -> &'static LocalKey<ManuallyDrop<Workspace>> {
        &WORKSPACE
    }

    /// Return a recycled atom from this workspace. The atom may have the same value as before.
    #[inline]
    pub fn new_atom(&self) -> RecycledAtom {
        if let Ok(mut a) = self.atom_buffer.try_borrow_mut() {
            if let Some(b) = a.pop() {
                b.into()
            } else {
                Atom::default().into()
            }
        } else {
            Atom::default().into() // very rare
        }
    }

    /// Create a new variable from a recycled atom from this workspace.
    #[inline]
    pub fn new_var(&self, id: Symbol) -> RecycledAtom {
        let mut owned = self.new_atom();
        owned.to_var(id);
        owned
    }

    /// Create a new number from a recycled atom from this workspace.
    #[inline]
    pub fn new_num<T: Into<Coefficient>>(&self, num: T) -> RecycledAtom {
        let mut owned = self.new_atom();
        owned.to_num(num.into());
        owned
    }

    pub fn return_atom(&self, atom: Atom) {
        if let Ok(mut a) = self.atom_buffer.try_borrow_mut() {
            a.push(atom);
        }
    }
}

#[derive(PartialEq, Eq, Debug, Hash, Clone)]
pub struct RecycledAtom(Atom);

impl From<Atom> for RecycledAtom {
    fn from(a: Atom) -> Self {
        RecycledAtom(a)
    }
}

impl std::fmt::Display for RecycledAtom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Default for RecycledAtom {
    fn default() -> Self {
        Self::new()
    }
}

impl RecycledAtom {
    /// Get a recycled atom from a thread-local workspace.
    #[inline]
    pub fn new() -> RecycledAtom {
        Workspace::get_local().with(|ws| ws.new_atom())
    }

    /// Wrap an atom so that it gets recycled upon dropping.
    pub fn wrap(atom: Atom) -> RecycledAtom {
        RecycledAtom(atom)
    }

    #[inline]
    pub fn new_var(id: Symbol) -> RecycledAtom {
        let mut owned = Self::new();
        owned.to_var(id);
        owned
    }

    /// Create a new number from a recycled atom from this workspace.
    #[inline]
    pub fn new_num<T: Into<Coefficient>>(num: T) -> RecycledAtom {
        let mut owned = Self::new();
        owned.to_num(num.into());
        owned
    }

    /// Yield the atom, which will now no longer be recycled upon dropping.
    pub fn into_inner(mut self) -> Atom {
        std::mem::replace(&mut self.0, Atom::Empty)
    }
}

impl Deref for RecycledAtom {
    type Target = Atom;

    fn deref(&self) -> &Atom {
        &self.0
    }
}

impl DerefMut for RecycledAtom {
    fn deref_mut(&mut self) -> &mut Atom {
        &mut self.0
    }
}

impl AsRef<Atom> for RecycledAtom {
    fn as_ref(&self) -> &Atom {
        self.deref()
    }
}

impl Drop for RecycledAtom {
    #[inline]
    fn drop(&mut self) {
        if let Atom::Empty = self.0 {
            return;
        }

        let _ = WORKSPACE.try_with(
            #[inline(always)]
            |ws| {
                if let Ok(mut a) = ws.atom_buffer.try_borrow_mut() {
                    if a.len() < Workspace::ATOM_BUFFER_MAX {
                        a.push(std::mem::replace(&mut self.0, Atom::Empty));
                    }
                }
            },
        );
    }
}
