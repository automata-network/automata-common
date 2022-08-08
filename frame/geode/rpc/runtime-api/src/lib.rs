#![cfg_attr(not(feature = "std"), no_std)]

pub use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
    pub trait GeodeRuntimeApi {
        fn unsigned_geode_ready(message: Vec<u8>, signature_raw_bytes: [u8; 64]) -> bool;
        fn unsigned_geode_finalizing(message: Vec<u8>, signature_raw_bytes: [u8; 64]) -> bool;
        fn unsigned_geode_finalized(message: Vec<u8>, signature_raw_bytes: [u8; 64]) -> bool;
        fn unsigned_geode_finalize_failed(message: Vec<u8>, signature_raw_bytes: [u8; 64]) -> bool;
    }
}
