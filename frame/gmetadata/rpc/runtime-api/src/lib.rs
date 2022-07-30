#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet_gmetadata::datastructures::{GmetadataKey, GmetadataQueryResult, HexBytes};
use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
    pub trait GmetadataRuntimeApi {
        fn query_with_index(
            index_key: Vec<GmetadataKey>,
            value_key: GmetadataKey,
            cursor: HexBytes,
            limit: u64
        ) -> GmetadataQueryResult;
    }
}
