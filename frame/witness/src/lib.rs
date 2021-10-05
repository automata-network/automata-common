// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use codec::{Decode, Encode, EncodeLike};
    pub use frame_support::{pallet_prelude::*, weights::GetDispatchInfo, PalletId, Parameter};
    use frame_system::{self as system, pallet_prelude::*};
    pub use sp_core::U256;
    use sp_runtime::traits::{AccountIdConversion, Dispatchable};
    use sp_runtime::RuntimeDebug;
    use sp_std::prelude::*;

    const DEFAULT_RELAYER_THRESHOLD: u32 = 1;

    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
    pub struct WorkSpace {

    }

    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
    pub enum ProposalStatus {
        Initiated,
        Approved,
        Rejected,
    }

   

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        /// Origin used to administer the pallet
        type AdminOrigin: EnsureOrigin<Self::Origin>;
        /// Proposed dispatchable call
        type Proposal: Parameter
            + Dispatchable<Origin = Self::Origin>
            + EncodeLike
            + GetDispatchInfo;
        /// The identifier for this chain.
        /// This must be unique and must not collide with existing IDs within a set of bridged chains.
        #[pallet::constant]
        type BridgeChainId: Get<BridgeChainId>;

        #[pallet::constant]
        type ProposalLifetime: Get<Self::BlockNumber>;
    }

    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Vote threshold has changed (new_threshold)
        RelayerThresholdChanged(u32),
        
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Relayer threshold not set
        ThresholdNotSet,
       
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::storage]
    #[pallet::getter(fn chains)]
    pub type ChainNonces<T> = StorageMap<_, Blake2_256, BridgeChainId, DepositNonce>;

    #[pallet::storage]
    #[pallet::getter(fn relayer_count)]
    pub type RelayerCount<T> = StorageValue<_, u32, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn votes)]
    pub type Votes<T: Config> = StorageDoubleMap<
        _,
        Blake2_256,
        BridgeChainId,
        Blake2_256,
        (DepositNonce, T::Proposal),
        ProposalVotes<T::AccountId, T::BlockNumber>,
    >;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Sets the vote threshold for proposals.
        ///
        /// This threshold is used to determine how many votes are required
        /// before a proposal is executed.
        ///
        /// # <weight>
        /// - O(1) lookup and insert
        /// # </weight>
        #[pallet::weight(195_000_000)]
        pub fn set_threshold(origin: OriginFor<T>, threshold: u32) -> DispatchResultWithPostInfo {
            T::AdminOrigin::ensure_origin(origin)?;
            // Self::set_relayer_threshold(threshold)
        }

        
    }

    impl<T: Config> Pallet<T> {
        // *** Utility methods ***

        /// Checks if who is a relayer
        pub fn is_relayer(who: &T::AccountId) -> bool {
            true
        }

    }
}
