// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use codec::{Decode, Encode, EncodeLike};
    pub use frame_support::{pallet_prelude::*, weights::GetDispatchInfo, PalletId, Parameter};
    use frame_system::{self as system, pallet_prelude::*};
    pub use sp_core::U256;

    use sp_runtime::{
        generic,
        traits::{
            self, AtLeast32Bit, AtLeast32BitUnsigned, BadOrigin, BlockNumberProvider, Bounded,
            CheckEqual, Dispatchable, Hash, Lookup, LookupError, MaybeDisplay, MaybeMallocSizeOf,
            MaybeSerializeDeserialize, Member, One, Saturating, SimpleBitOps, StaticLookup, Zero,
            AccountIdConversion,
        },
        DispatchError, Either, Perbill, RuntimeDebug,
    };

    use sp_std::prelude::*;

    const DEFAULT_RELAYER_THRESHOLD: u32 = 1;
    type EthAddress = [u8; 20];

    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
    pub struct WorkSpace<WorkSpaceId, ProposalId> {
        pub workspace_id: WorkSpaceId,
        pub max_proposal_id: ProposalId,
        pub erc20_contract: EthAddress,
        pub additional_data: Vec<u8>,
    }

    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
    pub struct WorkspaceAdditionalData {
        pub name: Vec<u8>,
        pub spec: Vec<u8>,
        pub contract: EthAddress,
        pub chainId: u32,
    }

    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
    pub struct Proposal<ProposalId, BlockNumber, BlockHeight> {
        pub proposal_id: ProposalId,
        pub author: EthAddress,
        pub start_block_number: BlockNumber,
        pub end_block_number: BlockNumber,
        pub snapshot: BlockHeight,
        pub data: Vec<u8>,
    }

    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
    pub struct ProposalData<BlockNumber, BlockHeight> {
        pub title: Vec<u8>,
        pub content: Vec<u8>,
        pub start_block_number: BlockNumber,
        pub end_block_number: BlockNumber,
        pub snapshot: BlockHeight,
        pub data: Vec<u8>,
    }

    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
    pub struct CallbackInfo {
        pub callback_type: Vec<u8>,
        pub contract: EthAddress,
        pub function_name: Vec<u8>,
        pub function_args: Vec<Vec<u8>>,
        pub function_vals: Vec<Vec<u8>>,
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
        
        /// workspace identity type
        type WorkSpaceId: Parameter
            + EncodeLike
            + AtLeast32BitUnsigned;

        /// workspace identity type
        type ChainId: Parameter
        + EncodeLike
        + AtLeast32BitUnsigned;

        /// proposal identity type
        type ProposalId: Parameter
        + EncodeLike
        + AtLeast32BitUnsigned;

        /// proposal identity type
        type BlockHeight: Parameter
        + EncodeLike
        + AtLeast32BitUnsigned;

        #[pallet::constant]
        type MaxWorkSpaceAdditionalDataLength: Get<u32>;

        #[pallet::constant]
        type MaxWorkSpaceNameLength: Get<u32>;

        #[pallet::constant]
        type MaxWorkSpaceSpecLength: Get<u32>;

        #[pallet::constant]
        type MaxProposalDataLength: Get<u32>;

        #[pallet::constant]
        type MaxProposalTitleLength: Get<u32>;

        #[pallet::constant]
        type MaxProposalContentLength: Get<u32>;

        #[pallet::constant]
        type MaxProposalOptionLength: Get<u32>;

        #[pallet::constant]
        type MaxProposalOptionDescLength: Get<u32>;

        #[pallet::constant]
        type MaxProposalCallBackLength: Get<u32>;

        #[pallet::constant]
        type MaxProposalCallBackFunctionNameLength: Get<u32>;

        #[pallet::constant]
        type MaxProposalCallBackFunctionArgsLength: Get<u32>;

        #[pallet::constant]
        type MaxProposalCallBackFunctionParameterLength: Get<u32>;

        #[pallet::constant]
        type MaxProposalCallBackFunctionValueLength: Get<u32>;
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

    // #[pallet::storage]
    // #[pallet::getter(fn chains)]
    // pub type ChainNonces<T> = StorageMap<_, Blake2_256, BridgeChainId, DepositNonce>;

    #[pallet::storage]
    #[pallet::getter(fn relayer_count)]
    pub type RelayerCount<T> = StorageValue<_, u32, ValueQuery>;

    // #[pallet::storage]
    // #[pallet::getter(fn votes)]
    // pub type Votes<T: Config> = StorageDoubleMap<
    //     _,
    //     Blake2_256,
    //     BridgeChainId,
    //     Blake2_256,
    //     (DepositNonce, T::Proposal),
    //     ProposalVotes<T::AccountId, T::BlockNumber>,
    // >;

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
        #[pallet::weight(0)]
        pub fn set_threshold(origin: OriginFor<T>, threshold: u32) -> DispatchResultWithPostInfo {
            T::AdminOrigin::ensure_origin(origin)?;
            // Self::set_relayer_threshold(threshold)
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn create_workspace(origin: OriginFor<T>, 
            max_proposal_id: u32,
            erc20_contract: EthAddress,
            additional_data: Vec<u8>,
            name: Vec<u8>,
            spec: Vec<u8>,
            contract: EthAddress,
            chainId: T::ChainId) -> DispatchResultWithPostInfo {

            ensure_signed(origin)?;
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn force_create_workspace(origin: OriginFor<T>, 
            max_proposal_id: u32,
            erc20_contract: EthAddress,
            additional_data: Vec<u8>,
            name: Vec<u8>,
            spec: Vec<u8>,
            contract: EthAddress,
            chainId: T::ChainId) -> DispatchResultWithPostInfo {

            ensure_root(origin)?;
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn force_remove_workspace(origin: OriginFor<T>, 
            workspace_id: T::WorkSpaceId) -> DispatchResultWithPostInfo {

            ensure_root(origin)?;
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn create_proposal(origin: OriginFor<T>, 
            author: EthAddress,
            start_block_number: T::BlockNumber,
            end_block_number: T::BlockNumber,
            snapshot: T::BlockHeight,
            data: Vec<u8>,
            proposalData: ProposalData<T::BlockNumber, T::BlockHeight>,
            callbackInfo: CallbackInfo) -> DispatchResultWithPostInfo {

            ensure_signed(origin)?;
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn force_create_proposal(origin: OriginFor<T>, 
            author: EthAddress,
            start_block_number: T::BlockNumber,
            end_block_number: T::BlockNumber,
            snapshot: T::BlockHeight,
            data: Vec<u8>,
            proposalData: ProposalData<T::BlockNumber, T::BlockHeight>,
            callbackInfo: CallbackInfo) -> DispatchResultWithPostInfo {


            ensure_root(origin)?;
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn force_remove_proposal(origin: OriginFor<T>, 
            proposal_id: T::WorkSpaceId) -> DispatchResultWithPostInfo {

            ensure_root(origin)?;
            Ok(().into())
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
