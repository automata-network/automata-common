// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use codec::{Decode, Encode, EncodeLike};
    pub use frame_support::{pallet_prelude::*, weights::GetDispatchInfo, PalletId, Parameter};
    use frame_system::pallet_prelude::*;
    pub use sp_core::U256;

    use sp_runtime::{
        traits::{AtLeast32BitUnsigned, Bounded},
        RuntimeDebug,
    };

    use sp_std::prelude::*;

    #[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
    pub enum ChainId {
        Ethereum,
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
    pub struct WorkSpace {
        pub erc20_contract: EthAddress,
        pub additional_data: Vec<u8>,
    }

    impl WorkSpace {
        pub fn new(erc20_contract: EthAddress, additional_data: Vec<u8>) -> Self {
            WorkSpace {
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
        pub chain_id: ChainId,
    }

    impl WorkspaceAdditionalData {
        pub fn new(name: Vec<u8>, spec: Vec<u8>, contract: EthAddress, chain_id: ChainId) -> Self {
            WorkspaceAdditionalData {
                name: name,
                spec: spec,
                contract: contract,
                chain_id: chain_id,
            }
        }
    }

    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
    pub struct Proposal<WorkspaceId, BlockNumber, BlockHeight> {
        pub workspace_id: WorkspaceId,
        pub author: EthAddress,
        pub start_block_number: BlockNumber,
        pub end_block_number: BlockNumber,
        pub snapshot: BlockHeight,
        pub data: Vec<u8>,
        pub proposal_data: ProposalData,
        pub callback_info: CallbackInfo,
    }

    impl<WorkspaceId, BlockNumber, BlockHeight> Proposal<WorkspaceId, BlockNumber, BlockHeight> {
        pub fn new(
            workspace_id: WorkspaceId,
            author: EthAddress,
            start_block_number: BlockNumber,
            end_block_number: BlockNumber,
            snapshot: BlockHeight,
            data: Vec<u8>,
            proposal_data: ProposalData,
            callback_info: CallbackInfo,
        ) -> Self {
            Proposal {
                workspace_id: workspace_id,
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
        pub privacy_level: PrivacyLevel,
        pub data: Vec<u8>,
    }

    impl ProposalData {
        pub fn new(
            title: Vec<u8>,
            content: Vec<u8>,
            options: Vec<Vec<u8>>,
            privacy_level: PrivacyLevel,
            data: Vec<u8>,
        ) -> Self {
            ProposalData {
                title: title,
                content: content,
                options: options,
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
        pub fn new(
            callback_type: CallbackType,
            contract: EthAddress,
            function_name: Vec<u8>,
            function_args: Vec<Vec<u8>>,
            function_vals: Vec<Vec<u8>>,
        ) -> Self {
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

        /// workspace identity type
        type WorkSpaceId: Parameter + EncodeLike + AtLeast32BitUnsigned + Default + Clone + Copy;

        /// proposal identity type
        type ProposalId: Parameter + EncodeLike + AtLeast32BitUnsigned + Default;

        /// block height for block chain like Ethereum
        type BlockHeight: Parameter + EncodeLike + AtLeast32BitUnsigned;

        #[pallet::constant]
        type MaxWorkSpaceAdditionalDataLength: Get<u32>;

        #[pallet::constant]
        type MaxWorkSpaceNameLength: Get<u32>;

        #[pallet::constant]
        type MaxWorkSpaceSpecLength: Get<u32>;

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
        NewWorkSpace(
            Option<T::AccountId>,
            T::WorkSpaceId,
            EthAddress,
            Vec<u8>,
            Vec<u8>,
            EthAddress,
            ChainId,
        ),
        /// Create a new proposal
        NewProposal(
            Option<T::AccountId>,
            T::ProposalId,
            T::WorkSpaceId,
            EthAddress,
            T::BlockNumber,
            T::BlockNumber,
            T::BlockHeight,
            Vec<u8>,
            ProposalData,
            CallbackInfo,
        ),
        /// Create a new vote
        NewVote(Option<T::AccountId>, T::ProposalId, u32),
        /// One proposal finalized
        ProposalFinalized(T::BlockNumber, T::ProposalId, u32),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// workspace id overflow
        WorkSpaceIdOverFlow,
        /// workspace id not created yet
        WorkSpaceNotExists,
        /// workspace additional data length too long
        InvalidWorkSpaceAdditionalDataLength,
        /// workspace name too long
        InvalidWorkSpaceNameLength,
        //// workspace spec lenth too long
        InvalidWorkSpaceSpecLength,
        /// invalid block number scope
        InvalidBlockNumberScope,
        /// proposal id overflow
        ProposalIdOverFlow,
        /// invalid propsal end block number
        InvalidProposalEndBlockNumber,
        /// proposal data
        InvalidProposalTitleLength,
        /// proposal content length too long
        InvalidProposalContentLength,
        /// proposal has too much options
        InvalidProposalOptionLength,
        /// proposal option description too long
        InvalidProposalOptionDescLength,
        /// proposal data length too long
        InvalidProposalDataLength,
        /// call back info
        InvalidCallBackFunctionNameLength,
        /// call back function arguments' length not match values
        CallBackFunctionArgsValsNotMatch,
        /// call back function parameter name too long
        InvalidCallBackFunctionParameterLength,
        /// call back function value too long
        InvalidCallBackFunctionValueLength,
        /// vote for not existed proposal
        ProposalNotExists,
        /// vote for expired proposal
        ProposalExpired,
        /// vote index out of scope of options
        InvalidVoteIndex,
    }

    /// workspace id for next new workspace
    #[pallet::storage]
    #[pallet::getter(fn current_work_space_id)]
    pub type CurrentWorkSpaceId<T: Config> = StorageValue<_, T::WorkSpaceId, ValueQuery>;

    /// proposal id for next new proposal
    #[pallet::storage]
    #[pallet::getter(fn current_proposal_id)]
    pub type CurrentProposalId<T: Config> = StorageValue<_, T::ProposalId, ValueQuery>;

    /// workspace map store all workspace data
    #[pallet::storage]
    #[pallet::getter(fn work_space_map)]
    pub type WorkSpaceMap<T: Config> =
        StorageMap<_, Blake2_256, T::WorkSpaceId, WorkSpace, OptionQuery>;

    /// worksapce additional data map store additional data for workspace
    #[pallet::storage]
    #[pallet::getter(fn work_space_additional_data_map)]
    pub type WorkspaceAdditionalDataMap<T: Config> =
        StorageMap<_, Blake2_256, T::WorkSpaceId, WorkspaceAdditionalData, OptionQuery>;

    /// proposal map store all proposals
    #[pallet::storage]
    #[pallet::getter(fn proposal_map)]
    pub type ProposalMap<T: Config> = StorageMap<
        _,
        Blake2_256,
        T::ProposalId,
        Proposal<T::WorkSpaceId, T::BlockNumber, T::BlockHeight>,
        OptionQuery,
    >;

    /// vote map store all votes for proposals
    #[pallet::storage]
    #[pallet::getter(fn vote_map)]
    pub type VoteMap<T: Config> = StorageMap<_, Blake2_256, T::ProposalId, Vec<u32>, ValueQuery>;

    /// proposal expiration map store the proposals according to end block number
    /// on finalize can get the expired proposals quickly
    #[pallet::storage]
    #[pallet::getter(fn proposal_expiration_map)]
    pub type ProposalExpirationMap<T: Config> =
        StorageMap<_, Blake2_256, T::BlockNumber, Vec<T::ProposalId>, ValueQuery>;

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        /// Block finalization to deal with all expired proposals
        fn on_finalize(block_number: BlockNumberFor<T>) {
            let proposal_ids = ProposalExpirationMap::<T>::get(block_number);

            // iterate the proposal
            for proposal_id in proposal_ids.iter() {
                let votes = VoteMap::<T>::get(proposal_id);
                let mut max_vote = 0;
                let mut max_vote_index = 0;

                // iterate votes for all options, find the maximum one
                for (index, item) in votes.iter().enumerate() {
                    if *item > max_vote {
                        max_vote = *item;
                        max_vote_index = index;
                    }
                }
                Self::deposit_event(Event::ProposalFinalized(
                    block_number,
                    proposal_id.clone(),
                    max_vote_index as u32,
                ));
            }

            // remove the entry for current block number
            ProposalExpirationMap::<T>::remove(block_number);
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// create workspace from account
        #[pallet::weight(0)]
        pub fn create_workspace(
            origin: OriginFor<T>,
            erc20_contract: EthAddress,
            additional_data: Vec<u8>,
            name: Vec<u8>,
            spec: Vec<u8>,
            contract: EthAddress,
            chain_id: ChainId,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            Self::add_workspace(
                Some(who),
                erc20_contract,
                additional_data,
                name.clone(),
                spec.clone(),
                contract,
                chain_id,
            )
        }

        /// force create workspace from root
        #[pallet::weight(0)]
        pub fn force_create_workspace(
            origin: OriginFor<T>,
            erc20_contract: EthAddress,
            additional_data: Vec<u8>,
            name: Vec<u8>,
            spec: Vec<u8>,
            contract: EthAddress,
            chain_id: ChainId,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            Self::add_workspace(
                None,
                erc20_contract,
                additional_data,
                name.clone(),
                spec.clone(),
                contract,
                chain_id,
            )
        }

        /// force remove workspace from root
        #[pallet::weight(0)]
        pub fn force_remove_workspace(
            origin: OriginFor<T>,
            workspace_id: T::WorkSpaceId,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            Self::remove_workspace(workspace_id);
            Ok(().into())
        }

        /// create proposal from account
        #[pallet::weight(0)]
        pub fn create_proposal(
            origin: OriginFor<T>,
            workspace_id: T::WorkSpaceId,
            author: EthAddress,
            start_block_number: T::BlockNumber,
            end_block_number: T::BlockNumber,
            snapshot: T::BlockHeight,
            data: Vec<u8>,
            proposal_data: ProposalData,
            callback_info: CallbackInfo,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Self::add_proposal(
                Some(who),
                workspace_id,
                author,
                start_block_number,
                end_block_number,
                snapshot,
                data,
                proposal_data,
                callback_info,
            )?;
            Ok(().into())
        }

        /// force create proposal from root
        #[pallet::weight(0)]
        pub fn force_create_proposal(
            origin: OriginFor<T>,
            workspace_id: T::WorkSpaceId,
            author: EthAddress,
            start_block_number: T::BlockNumber,
            end_block_number: T::BlockNumber,
            snapshot: T::BlockHeight,
            data: Vec<u8>,
            proposal_data: ProposalData,
            callback_info: CallbackInfo,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            Self::add_proposal(
                None,
                workspace_id,
                author,
                start_block_number,
                end_block_number,
                snapshot,
                data,
                proposal_data,
                callback_info,
            )?;
            Ok(().into())
        }

        /// force remove proposal from root
        #[pallet::weight(0)]
        pub fn force_remove_proposal(
            origin: OriginFor<T>,
            proposal_id: T::ProposalId,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;

            ProposalMap::<T>::remove(proposal_id);
            Ok(().into())
        }

        /// vote for a proposal with option index
        #[pallet::weight(0)]
        pub fn vote_for_proposal(
            origin: OriginFor<T>,
            proposal_id: T::ProposalId,
            index: u32,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            Self::append_vote(Some(who), proposal_id, index)?;

            Ok(().into())
        }

        /// force vote for a proposal from root
        #[pallet::weight(0)]
        pub fn force_vote_for_proposal(
            origin: OriginFor<T>,
            proposal_id: T::ProposalId,
            index: u32,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;

            Self::append_vote(None, proposal_id, index)?;

            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        /// add a new workspace
        pub fn add_workspace(
            who: Option<T::AccountId>,
            erc20_contract: EthAddress,
            additional_data: Vec<u8>,
            name: Vec<u8>,
            spec: Vec<u8>,
            contract: EthAddress,
            chain_id: ChainId,
        ) -> DispatchResultWithPostInfo {
            // valid check
            Self::ensure_valid_workspace(&additional_data, &name, &spec)?;
            let current_work_space_id = Self::current_work_space_id();

            // create new object
            let new_work_space = WorkSpace::new(erc20_contract.clone(), additional_data.clone());
            let new_work_space_addtional_data = WorkspaceAdditionalData::new(
                name.clone(),
                spec.clone(),
                contract.clone(),
                chain_id,
            );

            // insert data into map
            WorkSpaceMap::<T>::insert(current_work_space_id, new_work_space);
            WorkspaceAdditionalDataMap::<T>::insert(
                current_work_space_id,
                new_work_space_addtional_data,
            );

            // update work space id
            CurrentWorkSpaceId::<T>::put(current_work_space_id + 1_u32.into());
            // store event
            Self::deposit_event(Event::NewWorkSpace(
                who,
                current_work_space_id,
                erc20_contract,
                name,
                spec,
                contract,
                chain_id,
            ));

            Ok(().into())
        }

        /// add new proposal
        pub fn add_proposal(
            who: Option<T::AccountId>,
            workspace_id: T::WorkSpaceId,
            author: EthAddress,
            start_block_number: T::BlockNumber,
            end_block_number: T::BlockNumber,
            snapshot: T::BlockHeight,
            data: Vec<u8>,
            proposal_data: ProposalData,
            callback_info: CallbackInfo,
        ) -> DispatchResultWithPostInfo {
            // check end block number
            let current_block_number = <frame_system::Pallet<T>>::block_number();
            ensure!(
                current_block_number < end_block_number,
                Error::<T>::InvalidProposalEndBlockNumber
            );

            // check all parameters
            Self::ensure_valid_proposal(
                workspace_id,
                start_block_number,
                end_block_number,
                &data,
                &proposal_data,
                &callback_info,
            )?;

            // create new object
            let current_proposal_id = Self::current_proposal_id();
            let new_proposal = Proposal::new(
                workspace_id,
                author,
                start_block_number,
                end_block_number,
                snapshot.clone(),
                data.clone(),
                proposal_data.clone(),
                callback_info.clone(),
            );

            // insert data into storage
            ProposalMap::<T>::insert(current_proposal_id.clone(), new_proposal);
            ProposalExpirationMap::<T>::mutate(&end_block_number, |proposal_ids| {
                proposal_ids.push(current_proposal_id.clone())
            });

            // udpate proposal id
            CurrentProposalId::<T>::put(current_proposal_id.clone() + 1_u32.into());

            // emit event
            Self::deposit_event(Event::NewProposal(
                who,
                current_proposal_id,
                workspace_id,
                author,
                start_block_number,
                end_block_number,
                snapshot,
                data,
                proposal_data,
                callback_info,
            ));

            Ok(().into())
        }

        /// append new vote into vote records
        pub fn append_vote(
            who: Option<T::AccountId>,
            proposal_id: T::ProposalId,
            index: u32,
        ) -> DispatchResultWithPostInfo {
            let current_block_number = <frame_system::Pallet<T>>::block_number();

            // check vote parameter
            Self::ensure_valid_vote(current_block_number, proposal_id.clone(), index)?;

            // append new vote into records
            VoteMap::<T>::mutate(&proposal_id, |votes| votes[index as usize] += 1);

            // emit event
            Self::deposit_event(Event::NewVote(who, proposal_id, index));

            Ok(().into())
        }

        /// ensure vote is valid
        pub fn ensure_valid_vote(
            current_block_number: T::BlockNumber,
            proposal_id: T::ProposalId,
            index: u32,
        ) -> DispatchResultWithPostInfo {
            match ProposalMap::<T>::get(proposal_id) {
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
                }
                None => Err(Error::<T>::ProposalNotExists.into()),
            }
        }

        /// remove workspace from storage
        pub fn remove_workspace(workspace_id: T::WorkSpaceId) {
            // remove basic data
            WorkSpaceMap::<T>::remove(workspace_id);

            // remove additional data
            WorkspaceAdditionalDataMap::<T>::remove(workspace_id);
        }

        /// ensure valid workspace
        pub fn ensure_valid_workspace(
            additional_data: &Vec<u8>,
            name: &Vec<u8>,
            spec: &Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            ensure!(
                Self::current_work_space_id() != T::WorkSpaceId::max_value(),
                Error::<T>::WorkSpaceIdOverFlow
            );

            ensure!(
                additional_data.len() as u32 <= T::MaxWorkSpaceAdditionalDataLength::get(),
                Error::<T>::InvalidWorkSpaceAdditionalDataLength
            );

            ensure!(
                name.len() as u32 <= T::MaxWorkSpaceNameLength::get(),
                Error::<T>::InvalidWorkSpaceNameLength
            );

            ensure!(
                spec.len() as u32 <= T::MaxWorkSpaceSpecLength::get(),
                Error::<T>::InvalidWorkSpaceSpecLength
            );

            Ok(().into())
        }

        /// ensure proposal is valid
        pub fn ensure_valid_proposal(
            workspace_id: T::WorkSpaceId,
            start_block_number: T::BlockNumber,
            end_block_number: T::BlockNumber,
            data: &Vec<u8>,
            proposal_data: &ProposalData,
            callback_info: &CallbackInfo,
        ) -> DispatchResultWithPostInfo {
            ensure!(
                WorkSpaceMap::<T>::get(workspace_id).is_some(),
                Error::<T>::WorkSpaceNotExists
            );
            ensure!(
                Self::current_proposal_id() != T::ProposalId::max_value(),
                Error::<T>::ProposalIdOverFlow
            );

            ensure!(
                start_block_number < end_block_number,
                Error::<T>::InvalidBlockNumberScope
            );

            ensure!(
                data.len() as u32 <= T::MaxProposalDataLength::get(),
                Error::<T>::InvalidProposalDataLength
            );

            Self::ensure_valid_proposal_data(proposal_data)?;

            Self::ensure_valid_callback_info(callback_info)?;

            Ok(().into())
        }

        /// ensure proposal data valid
        pub fn ensure_valid_proposal_data(
            proposal_data: &ProposalData,
        ) -> DispatchResultWithPostInfo {
            ensure!(
                proposal_data.title.len() as u32 <= T::MaxProposalTitleLength::get(),
                Error::<T>::InvalidProposalTitleLength
            );

            ensure!(
                proposal_data.content.len() as u32 <= T::MaxProposalContentLength::get(),
                Error::<T>::InvalidProposalContentLength
            );

            ensure!(
                proposal_data.options.len() as u32 <= T::MaxProposalOptionLength::get(),
                Error::<T>::InvalidProposalOptionLength
            );

            for option in proposal_data.options.iter() {
                ensure!(
                    option.len() as u32 <= T::MaxProposalOptionDescLength::get(),
                    Error::<T>::InvalidProposalOptionDescLength
                );
            }

            ensure!(
                proposal_data.data.len() as u32 <= T::MaxProposalDataLength::get(),
                Error::<T>::InvalidProposalDataLength
            );

            Ok(().into())
        }

        /// ensure callback info is valid
        pub fn ensure_valid_callback_info(
            callback_info: &CallbackInfo,
        ) -> DispatchResultWithPostInfo {
            ensure!(
                callback_info.function_name.len() as u32
                    <= T::MaxProposalCallBackFunctionNameLength::get(),
                Error::<T>::InvalidCallBackFunctionNameLength
            );

            ensure!(
                callback_info.function_args.len() == callback_info.function_vals.len(),
                Error::<T>::CallBackFunctionArgsValsNotMatch
            );

            for index in 0..callback_info.function_args.len() {
                ensure!(
                    callback_info.function_args[index].len() as u32
                        <= T::MaxProposalCallBackFunctionParameterLength::get(),
                    Error::<T>::InvalidCallBackFunctionParameterLength
                );

                ensure!(
                    callback_info.function_vals[index].len() as u32
                        <= T::MaxProposalCallBackFunctionValueLength::get(),
                    Error::<T>::InvalidCallBackFunctionValueLength
                );
            }

            Ok(().into())
        }
    }
}
