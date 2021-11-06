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

    #[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
    pub enum ChainId {
        Ethereum
    }

    #[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
    pub enum PrivacyLevel {
        Private,
        Medium,
        Public,
    }

    type EthAddress = [u8; 20];

    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
    pub struct WorkSpace<ProposalId> {
        pub max_proposal_id: ProposalId,
        pub erc20_contract: EthAddress,
        pub additional_data: Vec<u8>,
    }

    impl<ProposalId> WorkSpace<ProposalId> {
        pub fn new(max_proposal_id: ProposalId,
            erc20_contract: EthAddress,
            additional_data: Vec<u8>) -> Self {
                WorkSpace {
                    max_proposal_id: max_proposal_id,
                    erc20_contract: erc20_contract,
                    additional_data: additional_data,
                }

        }
    }

    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
    pub struct WorkspaceAdditionalData {
        pub name: Vec<u8>,
        pub spec: Vec<u8>,
        pub contract: EthAddress,
        pub chainId: ChainId,
    }

    impl WorkspaceAdditionalData {
        pub fn new(name: Vec<u8>,
            spec: Vec<u8>,
            contract: EthAddress,
            chainId: ChainId,) -> Self {
                WorkspaceAdditionalData {
                    name: name,
                    spec: spec,
                    contract: contract,
                    chainId: chainId,
                }
        }
    }

    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
    pub struct Proposal<BlockNumber, BlockHeight> {
        pub author: EthAddress,
        pub start_block_number: BlockNumber,
        pub end_block_number: BlockNumber,
        pub snapshot: BlockHeight,
        pub data: Vec<u8>,
    }

    impl<BlockNumber, BlockHeight> Proposal<BlockNumber, BlockHeight> {
        pub fn new(author: EthAddress,
            start_block_number: BlockNumber,
            end_block_number: BlockNumber,
            snapshot: BlockHeight,
            data: Vec<u8>,) -> Self {
                Proposal {
                author: author,
                start_block_number: start_block_number,
                end_block_number: end_block_number,
                snapshot: snapshot,
                data: data,
                }
            }
    }

    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
    pub struct ProposalData {
        pub title: Vec<u8>,
        pub content: Vec<u8>,
        pub options: Vec<Vec<u8>>,
        pub votes: Vec<u32>,
        pub privacy_level: PrivacyLevel,
        pub data: Vec<u8>,
    }

    impl ProposalData {
        pub fn new(title: Vec<u8>,
            content: Vec<u8>,
            options: Vec<Vec<u8>>,
            votes: Vec<u32>,
            privacy_level: PrivacyLevel,
            data: Vec<u8>,) -> Self {
                ProposalData {
                    title: title,
                    content: content,
                    options: options,
                    votes: votes,
                    privacy_level: privacy_level,
                    data: data,
                }
            }
    }

    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
    pub struct CallbackInfo {
        pub callback_type: Vec<u8>,
        pub contract: EthAddress,
        pub function_name: Vec<u8>,
        pub function_args: Vec<Vec<u8>>,
        pub function_vals: Vec<Vec<u8>>,
    }

    impl CallbackInfo {
        pub fn new(callback_type: Vec<u8>,
            contract: EthAddress,
            function_name: Vec<u8>,
            function_args: Vec<Vec<u8>>,
            function_vals: Vec<Vec<u8>>,) -> Self {
                CallbackInfo {
                    callback_type: callback_type,
                    contract: contract,
                    function_name: function_name,
                    function_args: function_args,
                    function_vals: function_vals,
                }
            }
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
            + AtLeast32BitUnsigned
            + Default
            + Clone
            + Copy;

        /// workspace identity type
        type ChainId: Parameter
            + EncodeLike
            + AtLeast32BitUnsigned
            + Default
            + Clone
            + Copy;

        /// proposal identity type
        type ProposalId: Parameter
        + EncodeLike
        + AtLeast32BitUnsigned
        + Default;

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
        NewWorkSpace(T::AccountId, T::WorkSpaceId, EthAddress, Vec<u8>, Vec<u8>, EthAddress, ChainId)
        
    }

    #[pallet::error]
    pub enum Error<T> {
        /// 
        WorkSpaceIdOverFlow,
        InvalidWorkSpaceAdditionalDataLength,
        InvalidWorkSpaceNameLength,
        InvalidWorkSpaceSpecLength,
       
    }

    // #[pallet::storage]
    // #[pallet::getter(fn chains)]
    // pub type ChainNonces<T> = StorageMap<_, Blake2_256, BridgeChainId, DepositNonce>;

    #[pallet::storage]
    #[pallet::getter(fn current_work_space_id)]
    pub type CurrentWorkSpaceId<T: Config> = StorageValue<_, T::WorkSpaceId, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn work_space_map)]
    pub type WorkSpaceMap<T: Config> = StorageMap<
        _,
        Blake2_256,
        T::WorkSpaceId,
        WorkSpace<T::ProposalId>,
        OptionQuery
    >;

    #[pallet::storage]
    #[pallet::getter(fn work_space_additional_data_map)]
    pub type WorkspaceAdditionalDataMap<T: Config> = StorageMap<
        _,
        Blake2_256,
        T::WorkSpaceId,
        WorkspaceAdditionalData,
        OptionQuery
    >;

    #[pallet::storage]
    #[pallet::getter(fn proposal_map)]
    pub type ProposalMap<T: Config> = StorageMap<
        _,
        Blake2_256,
        T::ProposalId,
        Proposal<T::BlockNumber, T::BlockNumber>,
        OptionQuery
    >;

    #[pallet::storage]
    #[pallet::getter(fn proposal_expiration_map)]
    pub type ProposalExpirationMap<T: Config> = StorageMap<
        _,
        Blake2_256,
        T::BlockNumber,
        Vec<Proposal<T::BlockNumber, T::BlockNumber>>,
        OptionQuery
    >;

    #[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		

		/// Block finalization
		fn on_finalize(_n: BlockNumberFor<T>) {

			// ProposalMap::<T>::retain(|key, proposal| {
            //     true
            // });
		}
	}


    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// 
        
        #[pallet::weight(0)]
        pub fn create_workspace(origin: OriginFor<T>, 
            max_proposal_id: T::ProposalId,
            erc20_contract: EthAddress,
            additional_data: Vec<u8>,
            name: Vec<u8>,
            spec: Vec<u8>,
            contract: EthAddress,
            chainId: ChainId) -> DispatchResultWithPostInfo {

            let who = ensure_signed(origin)?;

            // valid check
            Self::ensure_valid_workspace(&additional_data, &name, &spec)?;
            let current_work_space_id = Self::current_work_space_id();
            Self::add_workspace(max_proposal_id,
                erc20_contract,
                additional_data,
                name.clone(),
                spec.clone(),
                contract,
                chainId);
            
            // store event            
            Self::deposit_event(Event::NewWorkSpace(who, current_work_space_id, erc20_contract, name, spec, contract, chainId));

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
            proposalData: ProposalData,
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
            proposalData: ProposalData,
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

        #[pallet::weight(0)]
        pub fn vote_for_proposal(origin: OriginFor<T>, 
            proposal_id: T::WorkSpaceId) -> DispatchResultWithPostInfo {

            ensure_root(origin)?;
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn force_vote_for_proposal(origin: OriginFor<T>, 
            proposal_id: T::WorkSpaceId) -> DispatchResultWithPostInfo {

            ensure_root(origin)?;
            Ok(().into())
        }

        
    }

    impl<T: Config> Pallet<T> {
        // *** Utility methods ***

        /// Checks if who is a relayer
        pub fn ensure_valid_workspace( 
            additional_data: &Vec<u8>,
            name: &Vec<u8>,
            spec: &Vec<u8>) -> DispatchResultWithPostInfo {

            ensure!(Self::current_work_space_id() != T::WorkSpaceId::max_value(), Error::<T>::WorkSpaceIdOverFlow);
            ensure!((additional_data.len() as u32) < T::MaxWorkSpaceAdditionalDataLength::get(), Error::<T>::InvalidWorkSpaceAdditionalDataLength);
            ensure!((name.len() as u32) < T::MaxWorkSpaceNameLength::get(), Error::<T>::InvalidWorkSpaceNameLength);
            ensure!((spec.len() as u32) < T::MaxWorkSpaceSpecLength::get(), Error::<T>::InvalidWorkSpaceSpecLength);

            Ok(().into())
        }

        pub fn add_workspace(max_proposal_id: T::ProposalId,
            erc20_contract: EthAddress,
            additional_data: Vec<u8>,
            name: Vec<u8>,
            spec: Vec<u8>,
            contract: EthAddress,
            chainId: ChainId) {
                // create new object
                let current_work_space_id = Self::current_work_space_id();


                let new_work_space = WorkSpace::new(max_proposal_id,
                    erc20_contract.clone(),
                    additional_data.clone());
                let new_work_space_addtional_data = WorkspaceAdditionalData::new(
                    name.clone(), spec.clone(), contract.clone(), chainId
                );

                // insert data into map
                WorkSpaceMap::<T>::insert(current_work_space_id, new_work_space);
                WorkspaceAdditionalDataMap::<T>::insert(current_work_space_id, new_work_space_addtional_data);

                // update work space id
                CurrentWorkSpaceId::<T>::put(current_work_space_id + 1_u32.into());
            }

        pub fn is_valid_proposal(who: &T::AccountId) -> bool {
            true
        }

        pub fn is_valid_vote(who: &T::AccountId) -> bool {
            true
        }

        pub fn is_valid_call_back(who: &T::AccountId) -> bool {
            true
        }

    }
}
