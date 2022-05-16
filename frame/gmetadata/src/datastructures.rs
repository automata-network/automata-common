use codec::{Decode, Encode};
use sp_runtime::RuntimeDebug;

#[cfg(feature = "std")]
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

use sp_std::prelude::*;

pub type GmetadataNamespaceName = Vec<u8>;

#[cfg(feature = "std")]
#[derive(Default, PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct HexBytes(Vec<u8>);

#[cfg(not(feature = "std"))]
#[derive(Default, PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct HexBytes(Vec<u8>);

impl From<&str> for HexBytes {
    fn from(val: &str) -> HexBytes {
        Self(val.into())
    }
}

impl HexBytes {
    pub fn new() -> Self {
        Self(Vec::new())
    }
}

impl From<Vec<u8>> for HexBytes {
    fn from(val: Vec<u8>) -> HexBytes {
        Self(val.into())
    }
}

impl From<&[u8]> for HexBytes {
    fn from(val: &[u8]) -> HexBytes {
        Self(val.into())
    }
}

impl From<HexBytes> for Vec<u8> {
    fn from(val: HexBytes) -> Vec<u8> {
        val.0
    }
}

#[cfg(feature = "std")]
impl<'de> Deserialize<'de> for HexBytes {
    fn deserialize<D>(deserializer: D) -> Result<HexBytes, D::Error>
    where
        D: Deserializer<'de>,
    {
        let str = String::deserialize(deserializer)?;
        let result: Vec<u8> = if str.starts_with("0x") {
            hex::decode(&str[2..]).map_err(|e| D::Error::custom(format!("{}", e)))?
        } else {
            str.into()
        };
        Ok(result.into())
    }
}

#[cfg(feature = "std")]
impl Serialize for HexBytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let val = format!("0x{}", hex::encode(&self.0));
        serializer.serialize_str(&val)
    }
}

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

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct GmetadataKey {
    pub ns: u64,         // namespace id
    pub table: HexBytes, // table name
    pub pk: HexBytes,    // primary key
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Default, PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct GmetadataQueryResult {
    pub list: Vec<HexBytes>,

    // cursor for fetch next batch, empty means reach the end
    pub cursor: HexBytes,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub enum GmetadataWriteOp {
    SetValue(GmetadataKey, Vec<u8>),
    RemoveValue(GmetadataKey),
    AddIndex(GmetadataKey, Vec<u8>),
    RemoveIndex(GmetadataKey, Vec<u8>),
}
