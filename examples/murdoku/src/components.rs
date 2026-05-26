//! Murdoku game components using cougr-core's ComponentTrait

use cougr_core::component::{ComponentStorage, ComponentTrait};
use soroban_sdk::{contracttype, symbol_short, Bytes, Env, Symbol, Vec, String};

/// A single pre-filled cell clue in the puzzle grid.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Clue {
    pub row: u32,
    pub col: u32,
    pub suspect_idx: u32,
}

/// Metadata associated with a Murdoku puzzle.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PuzzleMetadata {
    pub name: String,
    pub difficulty: String,
}

/// Grid size component representing the grid's side length.
#[derive(Clone, Debug, PartialEq)]
pub struct GridSize {
    pub size: u32,
}

impl ComponentTrait for GridSize {
    fn component_type() -> Symbol {
        symbol_short!("gridsize")
    }

    fn serialize(&self, env: &Env) -> Bytes {
        env.to_xdr(&self.size)
    }

    fn deserialize(env: &Env, data: &Bytes) -> Option<Self> {
        let size = env.from_xdr(data).ok()?;
        Some(Self { size })
    }

    fn default_storage() -> ComponentStorage {
        ComponentStorage::Table
    }
}

/// Suspects component storing the list of names.
#[derive(Clone, Debug, PartialEq)]
pub struct Suspects {
    pub list: Vec<String>,
}

impl ComponentTrait for Suspects {
    fn component_type() -> Symbol {
        symbol_short!("suspects")
    }

    fn serialize(&self, env: &Env) -> Bytes {
        env.to_xdr(&self.list)
    }

    fn deserialize(env: &Env, data: &Bytes) -> Option<Self> {
        let list = env.from_xdr(data).ok()?;
        Some(Self { list })
    }

    fn default_storage() -> ComponentStorage {
        ComponentStorage::Table
    }
}

/// Clues component storing the list of pre-filled cells.
#[derive(Clone, Debug, PartialEq)]
pub struct Clues {
    pub list: Vec<Clue>,
}

impl ComponentTrait for Clues {
    fn component_type() -> Symbol {
        symbol_short!("clues")
    }

    fn serialize(&self, env: &Env) -> Bytes {
        env.to_xdr(&self.list)
    }

    fn deserialize(env: &Env, data: &Bytes) -> Option<Self> {
        let list = env.from_xdr(data).ok()?;
        Some(Self { list })
    }

    fn default_storage() -> ComponentStorage {
        ComponentStorage::Table
    }
}

/// Solution component storing the full flat Latin square solution.
#[derive(Clone, Debug, PartialEq)]
pub struct Solution {
    pub grid: Vec<u32>,
}

impl ComponentTrait for Solution {
    fn component_type() -> Symbol {
        symbol_short!("solution")
    }

    fn serialize(&self, env: &Env) -> Bytes {
        env.to_xdr(&self.grid)
    }

    fn deserialize(env: &Env, data: &Bytes) -> Option<Self> {
        let grid = env.from_xdr(data).ok()?;
        Some(Self { grid })
    }

    fn default_storage() -> ComponentStorage {
        ComponentStorage::Table
    }
}

/// Metadata component storing description/name.
#[derive(Clone, Debug, PartialEq)]
pub struct Metadata {
    pub meta: PuzzleMetadata,
}

impl ComponentTrait for Metadata {
    fn component_type() -> Symbol {
        symbol_short!("metadata")
    }

    fn serialize(&self, env: &Env) -> Bytes {
        env.to_xdr(&self.meta)
    }

    fn deserialize(env: &Env, data: &Bytes) -> Option<Self> {
        let meta = env.from_xdr(data).ok()?;
        Some(Self { meta })
    }

    fn default_storage() -> ComponentStorage {
        ComponentStorage::Table
    }
}
