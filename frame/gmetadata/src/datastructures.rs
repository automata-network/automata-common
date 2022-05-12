use codec::{Decode, Encode};
use sp_runtime::RuntimeDebug;

use sp_std::prelude::*;

pub type GmetadataNamespaceName = Vec<u8>;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct GmetadataNamespaceInfo<AccountId> {
    pub id: u64,
    pub name: Vec<u8>,
    pub owners: Vec<AccountId>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct GmetadataValueInfo {
    pub data: Vec<u8>,
    pub update_time: u64,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct GmetadataIndexInfo {
    pub data: Vec<Vec<u8>>,
    pub update_time: u64,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct GmetadataKey {
    pub ns: u64,
    pub table: Vec<u8>,
    pub pk: Vec<u8>,
}

#[derive(Default, PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct GmetadataQueryResult {
    pub list: Vec<Vec<u8>>,
    pub cursor: Vec<u8>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub enum WriteOp {
    SetValue(GmetadataKey, Vec<u8>),
    RemoveValue(GmetadataKey),
    AddIndex(GmetadataKey, Vec<u8>),
    RemoveIndex(GmetadataKey, Vec<u8>),
}