#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use sp_runtime::traits::MaybeDisplay;
pub use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
    pub trait AttestorRuntimeApi<AccountId> where
        AccountId: Codec + MaybeDisplay
    {
        fn attestor_list() -> Vec<(Vec<u8>, Vec<u8>, u32)>;
        fn attestor_attested_appids(attestor: AccountId) -> Vec<AccountId>;
        fn unsigned_attestor_heartbeat(message: Vec<u8>, signature_raw_bytes: [u8; 64]) -> bool;
    }
}
