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
            AccountIdConversion, SaturatedConversion,
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

    #[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
    pub enum ProposalStatus {
        OK,
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
        pub proposal_data: ProposalData,
        pub callback_info: CallbackInfo,
    }

    impl<BlockNumber, BlockHeight> Proposal<BlockNumber, BlockHeight> {
        pub fn new(author: EthAddress,
            start_block_number: BlockNumber,
            end_block_number: BlockNumber,
            snapshot: BlockHeight,
            data: Vec<u8>,
            proposal_data: ProposalData,
            callback_info: CallbackInfo,) -> Self {
                Proposal {
                author: author,
                start_block_number: start_block_number,
                end_block_number: end_block_number,
                snapshot: snapshot,
                data: data,
                proposal_data: proposal_data,
                callback_info: callback_info,
                }
            }
    }

    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
    pub struct ProposalData {
        pub title: Vec<u8>,
        pub content: Vec<u8>,
        pub options: Vec<Vec<u8>>,
        // pub votes: Vec<u32>,
        pub privacy_level: PrivacyLevel,
        pub data: Vec<u8>,
    }

    impl ProposalData {
        pub fn new(title: Vec<u8>,
            content: Vec<u8>,
            options: Vec<Vec<u8>>,
            // votes: Vec<u32>,
            privacy_level: PrivacyLevel,
            data: Vec<u8>,) -> Self {
                ProposalData {
                    title: title,
                    content: content,
                    options: options,
                    // votes: votes,
                    privacy_level: privacy_level,
                    data: data,
                }
            }
    }

    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
    pub struct CallbackInfo {
        pub callback_type: CallbackType,
        pub contract: EthAddress,
        pub function_name: Vec<u8>,
        pub function_args: Vec<Vec<u8>>,
        pub function_vals: Vec<Vec<u8>>,
    }

    impl CallbackInfo {
        pub fn new(callback_type: CallbackType,
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

    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
    pub enum CallbackType {
        Solitidy,
        Ink,
        Pallet,
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

        
        /// proposal data
        #[pallet::constant]
        type MaxProposalTitleLength: Get<u32>;

        #[pallet::constant]
        type MaxProposalContentLength: Get<u32>;

        #[pallet::constant]
        type MaxProposalOptionLength: Get<u32>;

        #[pallet::constant]
        type MaxProposalOptionDescLength: Get<u32>;

        #[pallet::constant]
        type MaxProposalDataLength: Get<u32>;

        /// call back info
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
        /// Create a new work space
        NewWorkSpace(Option<T::AccountId>, T::WorkSpaceId, EthAddress, Vec<u8>, Vec<u8>, EthAddress, ChainId),
        /// Create a new proposal
        NewProposal(Option<T::AccountId>, T::ProposalId, EthAddress, T::BlockNumber, T::BlockNumber, T::BlockHeight, Vec<u8>, ProposalData, CallbackInfo),
        /// 
        NewVote(Option<T::AccountId>, T::ProposalId, u32),

        ///
        ProposalFinalized(T::BlockNumber, T::ProposalId, u32),

    }

    #[pallet::error]
    pub enum Error<T> {
        /// workspace id overflow
        WorkSpaceIdOverFlow,
        InvalidWorkSpaceAdditionalDataLength,
        InvalidWorkSpaceNameLength,
        InvalidWorkSpaceSpecLength,
        InvalidBlockNumberScope,

        /// proposal 
        ProposalIdOverFlow,
        InvalidProposalEndBlockNumber,

        /// proposal data
        InvalidProposalTitleLength,
        InvalidProposalContentLength,
        InvalidProposalOptionLength,
        InvalidProposalOptionDescLength,
        InvalidProposalDataLength,

        /// call back info
        InvalidCallBackFunctionNameLength,
        CallBackFunctionArgsValsNotMatch,
        InvalidCallBackFunctionParameterLength,
        InvalidCallBackFunctionValueLength,

        /// vote
        ProposalNotExists,
        ProposalExpired,
        InvalidVoteIndex,

       
    }

    // #[pallet::storage]
    // #[pallet::getter(fn chains)]
    // pub type ChainNonces<T> = StorageMap<_, Blake2_256, BridgeChainId, DepositNonce>;

    #[pallet::storage]
    #[pallet::getter(fn current_work_space_id)]
    pub type CurrentWorkSpaceId<T: Config> = StorageValue<_, T::WorkSpaceId, ValueQuery>;


    #[pallet::storage]
    #[pallet::getter(fn current_proposal_id)]
    pub type CurrentProposalId<T: Config> = StorageValue<_, T::ProposalId, ValueQuery>;

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
        Proposal<T::BlockNumber, T::BlockHeight>,
        OptionQuery
    >;

    #[pallet::storage]
    #[pallet::getter(fn vote_map)]
    pub type VoteMap<T: Config> = StorageMap<
        _,
        Blake2_256,
        T::ProposalId,
        Vec<u32>,
        ValueQuery
    >;

    #[pallet::storage]
    #[pallet::getter(fn proposal_expiration_map)]
    pub type ProposalExpirationMap<T: Config> = StorageMap<
        _,
        Blake2_256,
        T::BlockNumber,
        Vec<T::ProposalId>,
        ValueQuery,
    >;

    #[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		

		/// Block finalization
		fn on_finalize(block_number: BlockNumberFor<T>) {

			let proposalIds = ProposalExpirationMap::<T>::get(block_number);
            for proposal_id in proposalIds.iter() {
                let votes = VoteMap::<T>::get(proposal_id);
                let mut max_vote = 0;
                let mut max_vote_index = 0;

                for (index, item) in votes.iter().enumerate() {
                    if *item > max_vote {
                        max_vote = *item;
                        max_vote_index = index;
                    }
                }
                Self::deposit_event(Event::ProposalFinalized(block_number, proposal_id.clone(), max_vote_index as u32,));
            }
            
            ProposalExpirationMap::<T>::remove(block_number);
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

            Self::add_workspace(Some(who),
                max_proposal_id,
                erc20_contract,
                additional_data,
                name.clone(),
                spec.clone(),
                contract,
                chainId)
        }

        #[pallet::weight(0)]
        pub fn force_create_workspace(origin: OriginFor<T>, 
            max_proposal_id:  T::ProposalId,
            erc20_contract: EthAddress,
            additional_data: Vec<u8>,
            name: Vec<u8>,
            spec: Vec<u8>,
            contract: EthAddress,
            chainId: ChainId) -> DispatchResultWithPostInfo {

            ensure_root(origin)?;
            Self::add_workspace(None,
                max_proposal_id,
                erc20_contract,
                additional_data,
                name.clone(),
                spec.clone(),
                contract,
                chainId)
        }

        #[pallet::weight(0)]
        pub fn force_remove_workspace(origin: OriginFor<T>, 
            workspace_id: T::WorkSpaceId) -> DispatchResultWithPostInfo {

            ensure_root(origin)?;
            Self::remove_workspace(workspace_id);
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

            let who = ensure_signed(origin)?;
            Self::add_proposal(Some(who),
                author, 
                start_block_number, 
                end_block_number, 
                snapshot, 
                data, 
                proposalData, 
                callbackInfo)?;
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
            Self::add_proposal(None,
                author, 
                start_block_number, 
                end_block_number, 
                snapshot, 
                data, 
                proposalData, 
                callbackInfo)?;
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn force_remove_proposal(origin: OriginFor<T>, 
            proposal_id: T::ProposalId) -> DispatchResultWithPostInfo {

            ensure_root(origin)?;

            ProposalMap::<T>::remove(proposal_id);
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn vote_for_proposal(origin: OriginFor<T>, 
            proposal_id: T::ProposalId,
            index: u32) -> DispatchResultWithPostInfo {

            let who = ensure_signed(origin)?;
            let current_block_number = <frame_system::Pallet<T>>::block_number();

            Self::append_vote(Some(who), current_block_number, proposal_id, index);

            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn force_vote_for_proposal(origin: OriginFor<T>, 
            proposal_id: T::ProposalId) -> DispatchResultWithPostInfo {

            ensure_root(origin)?;
            Ok(().into())
        }

        
    }

    impl<T: Config> Pallet<T> {
        // *** Utility methods ***

        pub fn append_vote(who: Option<T::AccountId>,
            current_block_number: T::BlockNumber,
            proposalId: T::ProposalId,
            index: u32,
        ) {
            Self::ensure_valid_vote(current_block_number,
                proposalId.clone(), 
                index);

            VoteMap::<T>::mutate(&proposalId, |votes| {
                votes[index as usize] += 1
            });
        }

        /// add a new workspace
        pub fn add_workspace(who: Option<T::AccountId>,
            max_proposal_id: T::ProposalId,
            erc20_contract: EthAddress,
            additional_data: Vec<u8>,
            name: Vec<u8>,
            spec: Vec<u8>,
            contract: EthAddress,
            chainId: ChainId) -> DispatchResultWithPostInfo {

            // valid check
            Self::ensure_valid_workspace(&additional_data, &name, &spec)?;
            let current_work_space_id = Self::current_work_space_id();

            // create new object

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
            // store event            
            Self::deposit_event(Event::NewWorkSpace(who, current_work_space_id, erc20_contract, name, spec, contract, chainId));

            Ok(().into())
        }


        pub fn add_proposal(who: Option<T::AccountId>, 
            author: EthAddress,
            start_block_number: T::BlockNumber,
            end_block_number: T::BlockNumber,
            snapshot: T::BlockHeight,
            data: Vec<u8>,
            proposalData: ProposalData,
            callbackInfo: CallbackInfo) -> DispatchResultWithPostInfo {
            // check end block number
            let current_block_number = <frame_system::Pallet<T>>::block_number();
            ensure!(current_block_number < end_block_number, Error::<T>::InvalidProposalEndBlockNumber);

            Self::ensure_valid_proposal(author, 
                start_block_number, 
                end_block_number, 
                &snapshot, 
                &data, 
                &proposalData, 
                &callbackInfo);

            let current_proposal_id = Self::current_proposal_id();
            let new_proposal = Proposal::new(author, 
                start_block_number, 
                end_block_number, 
                snapshot.clone(), 
                data.clone(), 
                proposalData.clone(), 
                callbackInfo.clone());
            
            ProposalMap::<T>::insert(current_proposal_id.clone(), new_proposal);
            ProposalExpirationMap::<T>::mutate(&end_block_number, |proposalIds| {
                proposalIds.push(current_proposal_id.clone())
                
            });

            CurrentProposalId::<T>::put(current_proposal_id.clone() + 1_u32.into());

            Self::deposit_event(Event::NewProposal(who,
                current_proposal_id, 
                author,
                start_block_number, 
                end_block_number, 
                snapshot, 
                data, 
                proposalData, 
                callbackInfo));

            Ok(().into())
       }

        pub fn ensure_valid_vote(current_block_number: T::BlockNumber,
            proposalId: T::ProposalId,
            index: u32) -> DispatchResultWithPostInfo {
            
            match ProposalMap::<T>::get(proposalId) {
                Some(proposal) => {
                    if proposal.end_block_number < current_block_number {
                        Err(Error::<T>::ProposalExpired.into())
                    } else {
                        if proposal.proposal_data.options.len() as u32 >= index {
                            Err(Error::<T>::InvalidVoteIndex.into())
                        } else {
                            Ok(().into()) 
                        }
                    }
                },
                None => Err(Error::<T>::ProposalNotExists.into()),
            }
        }

        /// remove workspace from storage
        pub fn remove_workspace(workspace_id: T::WorkSpaceId) {
            WorkSpaceMap::<T>::remove(workspace_id);
            WorkspaceAdditionalDataMap::<T>::remove(workspace_id);
        }

        /// ensure valid workspace
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

        pub fn ensure_valid_proposal(author: EthAddress,
            start_block_number: T::BlockNumber,
            end_block_number: T::BlockNumber,
            snapshot: &T::BlockHeight,
            data: &Vec<u8>,
            proposalData: &ProposalData,
            callbackInfo: &CallbackInfo) -> DispatchResultWithPostInfo {
            ensure!(Self::current_proposal_id() != T::ProposalId::max_value(), Error::<T>::ProposalIdOverFlow);

            ensure!(start_block_number < end_block_number, Error::<T>::InvalidBlockNumberScope);

            ensure!(data.len() as u32 > T::MaxProposalDataLength::get(), Error::<T>::WorkSpaceIdOverFlow);

            Self::ensure_valid_proposal_data(proposalData);
            Self::ensure_valid_callback_info(callbackInfo);

            Ok(().into())
        }

        /// ensure proposal data valid
        pub fn ensure_valid_proposal_data(proposalData: &ProposalData) -> DispatchResultWithPostInfo {
            ensure!(proposalData.title.len() as u32 <= T::MaxProposalTitleLength::get(), Error::<T>::InvalidProposalTitleLength);
            ensure!(proposalData.content.len() as u32 <= T::MaxProposalContentLength::get(), Error::<T>::InvalidProposalContentLength);
            ensure!(proposalData.options.len() as u32 <= T::MaxProposalOptionLength::get(), Error::<T>::InvalidProposalOptionLength);
            for option in proposalData.options.iter() {
                ensure!(option.len() as u32 <= T::MaxProposalOptionDescLength::get(), Error::<T>::InvalidProposalOptionDescLength);
            }
            ensure!(proposalData.data.len() as u32 <= T::MaxProposalDataLength::get(), Error::<T>::InvalidProposalDataLength);

            Ok(().into())
        }

        /// ensure callback info is valid
        pub fn ensure_valid_callback_info(callback_info: &CallbackInfo) -> DispatchResultWithPostInfo {
            ensure!(T::MaxProposalCallBackFunctionNameLength::get() >= callback_info.function_name.len() as u32, Error::<T>::InvalidCallBackFunctionNameLength);
            ensure!(callback_info.function_args.len() == callback_info.function_vals.len(), Error::<T>::CallBackFunctionArgsValsNotMatch);
            
            for index in 0..callback_info.function_args.len() {
                ensure!(T::MaxProposalCallBackFunctionParameterLength::get() >= callback_info.function_args[index].len() as u32, Error::<T>::InvalidCallBackFunctionParameterLength);
                ensure!(T::MaxProposalCallBackFunctionValueLength::get() >= callback_info.function_vals[index].len() as u32, Error::<T>::InvalidCallBackFunctionValueLength);
            }
            Ok(().into())
        }

    }
}
