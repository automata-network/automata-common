#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

pub mod datastructures;

#[frame_support::pallet]
pub mod pallet {
    use crate::datastructures::*;
    pub use crate::weights::WeightInfo;
    use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
    use frame_system::{ensure_root, ensure_signed, pallet_prelude::*};

    use frame_support::{
        ensure,
        traits::{Currency, ExistenceRequirement, Get, UnixTime},
    };
    use sp_runtime::{traits::Saturating, DispatchResult, SaturatedConversion};
    use sp_std::{collections::btree_set::BTreeSet, prelude::*};

    type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_timestamp::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type Currency: Currency<Self::AccountId>;
        type UnixTime: UnixTime;

        type MinDuration: Get<u64>;
        type MaxDuration: Get<u64>;
        type MaxOptionCount: Get<OptionIndex>;
        type MaxWorkspace: Get<u32>;
        type MaxStrategy: Get<u32>;

        type DAOPortalWeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn vote_fee)]
    pub type VoteFee<T> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn relayer)]
    pub type Relayer<T: Config> = StorageValue<_, T::AccountId, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn latest_chain_index)]
    pub type LatestChainIndex<T> = StorageValue<_, ChainIndex, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn chains)]
    pub type Chains<T: Config> = StorageMap<_, Blake2_128Concat, ChainIndex, Chain>;

    #[pallet::storage]
    #[pallet::getter(fn latest_project_id)]
    pub type LatestProjectId<T> = StorageValue<_, ProjectId, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn projects)]
    pub type Projects<T: Config> =
        StorageMap<_, Blake2_128Concat, ProjectId, Project<T::AccountId>>;

    #[pallet::storage]
    #[pallet::getter(fn latest_proposal_id)]
    pub type LatestProposalId<T: Config> =
        StorageMap<_, Blake2_128Concat, ProjectId, ProposalId, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn proposals)]
    pub type Proposals<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ProjectId,
        Blake2_128Concat,
        ProposalId,
        DAOProposal<T::AccountId>,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId", BalanceOf<T> = "Balance")]
    pub enum Event<T: Config> {
        /// Chain registered. \[chain_index\]
        ChainRegistered(ChainIndex),
        /// Relayer updated. \[relayer\]
        RelayerUpdated(T::AccountId),
        /// Vote fee updated. \[relayer\]
        VoteFeeUpdated(BalanceOf<T>),
        /// Project created. \[project_id\]
        ProjectCreated(ProjectId),
        /// Project updated. \[project_id\]
        ProjectUpdated(ProjectId),
        /// Proposal created. \[project_id, proposal_id\]
        ProposalCreated(ProjectId, ProposalId),
        /// Vote updated. \[project_id, proposal_id\]
        VoteUpdated(ProjectId, ProposalId),
    }

    #[pallet::error]
    pub enum Error<T> {
        Unknown,
        NotRelayer,
        InvalidChain,
        InvalidStrategy,
        InvalidProject,
        InvalidProposal,
        InvalidSenderOrigin,
        InvalidSender,
        InvalidVote,
        InvalidWorkspace,
        InvalidDuration,
        InvalidFrequency,
        InsufficientBalance,
        InvalidStatus,
        ConflictWithPrivacyLevel,
        DuplicateWorkspace,
        DuplicateStrategy,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(T::DAOPortalWeightInfo::register_chain())]
        pub fn register_chain(origin: OriginFor<T>, chain: Chain) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;

            let chain_index = Self::latest_chain_index().saturating_add(1);

            Chains::<T>::insert(chain_index, chain);
            LatestChainIndex::<T>::set(chain_index);

            Self::deposit_event(Event::ChainRegistered(chain_index));

            Ok(().into())
        }

        #[pallet::weight(T::DAOPortalWeightInfo::update_relayer())]
        pub fn update_relayer(
            origin: OriginFor<T>,
            relayer: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            Relayer::<T>::set(relayer.clone());
            Self::deposit_event(Event::RelayerUpdated(relayer));
            Ok(().into())
        }

        #[pallet::weight(T::DAOPortalWeightInfo::update_vote_fee())]
        pub fn update_vote_fee(
            origin: OriginFor<T>,
            fee: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            VoteFee::<T>::put(fee);
            Self::deposit_event(Event::VoteFeeUpdated(fee));
            Ok(().into())
        }

        #[pallet::weight(
			T::DAOPortalWeightInfo::add_project(&project.workspaces)
		)]
        pub fn add_project(
            _origin: OriginFor<T>,
            project: Project<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            let project_id = Self::latest_project_id().saturating_add(1);
            Self::check_workspace(&project)?;
            Projects::<T>::insert(project_id, project);
            LatestProjectId::<T>::put(project_id);

            Self::deposit_event(Event::ProjectCreated(project_id));

            Ok(().into())
        }

        #[pallet::weight(
			T::DAOPortalWeightInfo::update_project_direct(&project.workspaces).max(T::DAOPortalWeightInfo::update_project_relay(&project.workspaces))
		)]
        pub fn update_project(
            origin: OriginFor<T>,
            project_id: ProjectId,
            project: Project<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            ensure!(
                project_id <= Self::latest_project_id(),
                Error::<T>::InvalidProject
            );

            Projects::<T>::try_mutate(project_id, |p| -> DispatchResult {
                if let Some(ref mut p) = p {
                    if who != Self::relayer() {
                        ensure!(
                            CrossChainAccount::<T::AccountId>::Substrate(who) == p.owner,
                            Error::<T>::InvalidSenderOrigin
                        );
                    }

                    Self::check_workspace(&project)?;

                    *p = project;

                    Self::deposit_event(Event::ProjectUpdated(project_id));

                    Ok(())
                } else {
                    Err(Error::<T>::InvalidProposal.into())
                }
            })?;

            Ok(().into())
        }

        #[pallet::weight(
			T::DAOPortalWeightInfo::add_proposal_direct(proposal._option_count.into()).max(T::DAOPortalWeightInfo::add_proposal_relay(proposal._option_count.into()))
		)]
        pub fn add_proposal(
            origin: OriginFor<T>,
            project_id: ProjectId,
            proposal: DAOProposal<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            ensure!(proposal._option_count > 1, Error::<T>::InvalidProposal);
            ensure!(
                proposal._option_count <= T::MaxOptionCount::get(),
                Error::<T>::InvalidProposal
            );
            ensure!(
                project_id <= Self::latest_project_id(),
                Error::<T>::InvalidProject
            );
            ensure!(
                proposal._start.saturating_add(T::MinDuration::get()) <= proposal._end,
                Error::<T>::InvalidDuration
            );
            ensure!(
                proposal._start.saturating_add(T::MaxDuration::get()) >= proposal._end,
                Error::<T>::InvalidDuration
            );

            let who = ensure_signed(origin)?;

            let relayer = Self::relayer();
            if who != relayer {
                let mut update_count: u32 = 1;
                if proposal._privacy != PrivacyLevel::Opaque {
                    if let Some(f) = proposal._frequency {
                        ensure!(f > 0, Error::<T>::InvalidFrequency);
                        update_count = (proposal
                            ._end
                            .saturating_sub(proposal._start)
                            .saturating_sub(1)
                            / f)
                            .saturated_into::<u32>()
                            .saturating_add(update_count);
                    }
                }
                let fee = Self::vote_fee().saturating_mul(update_count.into());
                ensure!(
                    T::Currency::free_balance(&who) >= fee,
                    Error::<T>::InsufficientBalance
                );
                <T as Config>::Currency::transfer(
                    &who,
                    &relayer,
                    fee,
                    ExistenceRequirement::AllowDeath,
                )?;
            }

            let mut status = DAOProposalStatus::Pending;
            if proposal._start <= T::UnixTime::now().as_millis().saturated_into::<u64>() {
                status = DAOProposalStatus::Ongoing;
            }

            let mut proposal = proposal.clone();

            proposal.state = DAOProposalState {
                status: status,
                votes: vec![0.into(); proposal._option_count.into()],
                pub_voters: None,
                updates: 0,
            };
            let proposal_id = Self::latest_proposal_id(project_id).saturating_add(1);
            Proposals::<T>::insert(project_id, proposal_id, proposal);
            LatestProposalId::<T>::insert(project_id, proposal_id);

            Self::deposit_event(Event::ProposalCreated(project_id, proposal_id));

            Ok(().into())
        }

        #[pallet::weight(
			T::DAOPortalWeightInfo::update_vote(update.votes.len().saturated_into())
		)]
        pub fn update_vote(origin: OriginFor<T>, update: VoteUpdate) -> DispatchResultWithPostInfo {
            // TODO ensure the timing for geode update is valid
            let who = ensure_signed(origin)?;
            ensure!(who == Self::relayer(), Error::<T>::NotRelayer);
            ensure!(
                update.project <= Self::latest_project_id(),
                Error::<T>::InvalidProject
            );
            ensure!(
                update.proposal <= Self::latest_proposal_id(update.project),
                Error::<T>::InvalidProposal
            );
            Proposals::<T>::try_mutate(
                update.project,
                update.proposal,
                |proposal| -> DispatchResult {
                    if let Some(ref mut proposal) = proposal {
                        ensure!(
                            update.votes.len() == proposal.state.votes.len(),
                            Error::<T>::InvalidVote
                        );
                        if &proposal.state.status == &DAOProposalStatus::Closed {
                            return Err(Error::<T>::InvalidStatus.into());
                        } else {
                            let current = &T::UnixTime::now().as_millis().saturated_into::<u64>();
                            if current >= &proposal._start {
                                if current < &proposal._end {
                                    ensure!(
                                        proposal._privacy != PrivacyLevel::Opaque,
                                        Error::<T>::ConflictWithPrivacyLevel
                                    );
                                    proposal.state.status = DAOProposalStatus::Ongoing;
                                } else {
                                    proposal.state.status = DAOProposalStatus::Closed;
                                }
                            } else {
                                return Err(Error::<T>::InvalidStatus.into());
                            }
                            if let Some(_) = update.pub_voters {
                                ensure!(
                                    proposal._privacy == PrivacyLevel::Mixed
                                        || proposal._privacy == PrivacyLevel::Public,
                                    Error::<T>::ConflictWithPrivacyLevel
                                );
                            }
                            proposal.state.pub_voters = update.pub_voters;
                            proposal.state.votes = update.votes;
                            proposal.state.updates = proposal.state.updates.saturating_add(1);
                            Self::deposit_event(Event::VoteUpdated(
                                update.project,
                                update.proposal,
                            ));
                        }
                        Ok(())
                    } else {
                        Err(Error::<T>::InvalidProposal.into())
                    }
                },
            )?;

            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        pub fn get_projects() -> Vec<(ProjectId, Project<T::AccountId>)> {
            <Projects<T>>::iter().collect()
        }

        pub fn get_proposals(project: ProjectId) -> Vec<(ProposalId, DAOProposal<T::AccountId>)> {
            <Proposals<T>>::iter_prefix(project).collect()
        }

        pub fn get_all_proposals() -> Vec<(ProjectId, ProposalId, DAOProposal<T::AccountId>)> {
            <Proposals<T>>::iter().collect()
        }

        fn check_workspace(project: &Project<T::AccountId>) -> DispatchResult {
            let mut chain_index = 0;

            ensure!(&project.workspaces.len() > &0, Error::<T>::InvalidProject);

            let mut workspaces = BTreeSet::<ChainIndex>::new();

            for workspace in &project.workspaces {
                if chain_index == 0 {
                    chain_index = Self::latest_chain_index();
                }
                ensure!(workspace._chain <= chain_index, Error::<T>::InvalidChain);
                ensure!(
                    !workspaces.contains(&workspace._chain),
                    Error::<T>::DuplicateWorkspace
                );
                workspaces.insert(workspace._chain.clone());
                let mut strategy_count = 0;
                match Self::chains(workspace._chain) {
                    Some(c) => match c._protocol {
                        Protocol::Solidity => {
                            let mut strategies = BTreeSet::<SolidityStrategy>::new();
                            for strategy in &workspace.strategies {
                                if let Strategy::Solidity(strategy) = strategy {
                                    ensure!(
                                        !strategies.contains(&strategy),
                                        Error::<T>::DuplicateStrategy
                                    );
                                    strategies.insert(strategy.clone());
                                    strategy_count = strategy_count.saturating_add(1);
                                } else {
                                    return Err(Error::<T>::InvalidStrategy.into());
                                }
                            }
                        }
                        Protocol::Substrate => {
                            let mut strategies = BTreeSet::<SubstrateStrategy>::new();
                            for strategy in &workspace.strategies {
                                if let Strategy::Substrate(strategy) = strategy {
                                    ensure!(
                                        !strategies.contains(&strategy),
                                        Error::<T>::DuplicateStrategy
                                    );
                                    strategies.insert(strategy.clone());
                                    strategy_count = strategy_count.saturating_add(1);
                                } else {
                                    return Err(Error::<T>::InvalidStrategy.into());
                                }
                            }
                        }
                    },
                    None => {
                        return Err(Error::<T>::Unknown.into());
                    }
                }
                ensure!(strategy_count > 0, Error::<T>::InvalidWorkspace);
            }

            Ok(())
        }
    }
}
