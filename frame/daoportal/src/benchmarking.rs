#![cfg(feature = "runtime-benchmarks")]

use super::*;

use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_support::{ensure, pallet_prelude::*, traits::Get};
use frame_system::{Pallet as System, RawOrigin};

use sp_std::{convert::TryInto, prelude::*};

use super::Pallet as DAOPortal;
use datastructures::*;

// fn set_relayer<T: Config>(who: &T::AccountId) -> Result<(), &'static str> {
//     DAOPortal::<T>::update_relayer()
// }

fn generate_workspace_n_strategy<T: Config>(s: u32) -> Vec<Workspace> {
    let mut l = 0;
    let mut workspaces = Vec::new(); 
    while l < s {
        DAOPortal::<T>::register_chain(
            RawOrigin::Root.into(),
            Chain {
                _protocol: Protocol::Solidity,
            },
        );
        let mut workspace = Workspace {
            _chain: DAOPortal::<T>::latest_chain_index(),
            strategies: Vec::new(),
        };
        let mut strategies = Vec::new();
        let mut c = 0;
        while c < T::MaxStrategy::get() {
            let mut whole = [0u8; 20];
            let (_, left) = whole.split_at_mut(16);
            left.copy_from_slice(&c.to_be_bytes());
            strategies.push(Strategy::Solidity(SolidityStrategy::ERC20Balance(
                EthAddress::from(whole),
            )));
            l += 1;
            c += 1;
        }
        workspace.strategies = strategies;
        workspaces.push(workspace);
    }
    workspaces
}

fn prepare_direct<T: Config>(caller: &T::AccountId, relay: bool) {
    DAOPortal::<T>::register_chain(
        RawOrigin::Root.into(),
        Chain {
            _protocol: Protocol::Solidity,
        },
    );

    if relay {
        DAOPortal::<T>::update_relayer(RawOrigin::Root.into(), caller.clone());
    }

    DAOPortal::<T>::add_project(
        RawOrigin::Signed(caller.clone()).into(),
        Project {
            owner: CrossChainAccount::Substrate(caller.clone()),
            data: IpfsHash::default(),
            workspaces: vec![Workspace {
                _chain: DAOPortal::<T>::latest_chain_index(),
                strategies: vec![Strategy::Solidity(SolidityStrategy::ERC20Balance(
                    EthAddress::default(),
                ))],
            }],
        },
    );

    assert_eq!(DAOPortal::<T>::latest_chain_index(), 1);
}

benchmarks! {
    register_chain {}: register_chain(RawOrigin::Root, Chain {
        _protocol: Protocol::Solidity,
    })
    verify {
        assert_eq!(DAOPortal::<T>::latest_chain_index(), 1);
    }

    update_relayer {}: update_relayer(RawOrigin::Root, account("relayer", 0, 0))
    verify {
        assert_eq!(DAOPortal::<T>::relayer(), account("relayer", 0, 0));
    }

    update_vote_fee {}: update_vote_fee(RawOrigin::Root, 100u32.into())
    verify {
        assert_eq!(DAOPortal::<T>::vote_fee(), 100u32.into());
    }

    add_project {
        let s in 1 .. (T::MaxWorkspace::get() * T::MaxStrategy::get());

        let caller: T::AccountId = account("caller", 1, 0);

        let mut project = Project {
            owner: CrossChainAccount::Substrate(caller.clone()),
            data: IpfsHash::default(),
            workspaces: Vec::new(),
        };
    }: add_project(RawOrigin::Signed(caller.clone()), project.clone(), generate_workspace_n_strategy::<T>(s))
    verify {
        assert_eq!(DAOPortal::<T>::projects(1).unwrap(), project);
    }

    update_project_direct {
        let s in 1 .. (T::MaxWorkspace::get() * T::MaxStrategy::get());

        let caller: T::AccountId = account("caller", 1, 0);

        prepare_direct::<T>(&caller, false);

        let mut project = Project {
            owner: CrossChainAccount::Substrate(caller.clone()),
            data: IpfsHash::default(),
            workspaces: Vec::new(),
        };
    }: update_project(RawOrigin::Signed(caller.clone()), 1, project.clone(), Some(generate_workspace_n_strategy::<T>(s)))
    verify {
        assert_eq!(DAOPortal::<T>::projects(1).unwrap(), project);
    }

    update_project_relay {
        let s in 1 .. (T::MaxWorkspace::get() * T::MaxStrategy::get());

        let relayer: T::AccountId = account("relayer", 0, 0);

        prepare_direct::<T>(&relayer, true);

        let mut project = Project {
            owner: CrossChainAccount::Solidity(EthAddress::default()),
            data: IpfsHash::default(),
            workspaces: Vec::new(),
        };

    }: update_project(RawOrigin::Signed(relayer.clone()), 1, project.clone(), Some(generate_workspace_n_strategy::<T>(s)))
    verify {
        assert_eq!(DAOPortal::<T>::projects(1).unwrap(), project);
    }

    add_proposal_direct {
        let s in 2 .. T::MaxOptionCount::get().into();

        let caller: T::AccountId = account("caller", 1, 0);

        prepare_direct::<T>(&caller, false);
    }: add_proposal(RawOrigin::Signed(caller.clone()), 1, Proposal {
        _author: CrossChainAccount::Substrate(caller.clone()),
        _voting_format: VotingFormat::SingleChoice,
        _option_count: s.try_into().unwrap(),
        _data: IpfsHash::default(),
        _privacy: PrivacyLevel::Mixed,
        _start: 0,
        _end: 7600000,
        _frequency: Some(3600000),
        state: ProposalState::default()
    })
    verify {
        assert_eq!(DAOPortal::<T>::latest_proposal_id(1), 1);
    }

    add_proposal_relay {
        let s in 2 .. T::MaxOptionCount::get().into();

        let relayer: T::AccountId = account("relayer", 0, 0);

        prepare_direct::<T>(&relayer, true);
    }: add_proposal(RawOrigin::Signed(relayer.clone()), 1, Proposal {
        _author: CrossChainAccount::Solidity(EthAddress::default()),
        _voting_format: VotingFormat::SingleChoice,
        _option_count: s.try_into().unwrap(),
        _data: IpfsHash::default(),
        _privacy: PrivacyLevel::Mixed,
        _start: 0,
        _end: 7600000,
        _frequency: Some(3600000),
        state: ProposalState::default()
    })
    verify {
        assert_eq!(DAOPortal::<T>::latest_proposal_id(1), 1);
    }

    update_vote {
        let s in 2 .. T::MaxOptionCount::get().into();

        let relayer: T::AccountId = account("relayer", 0, 0);

        prepare_direct::<T>(&relayer, true);

        DAOPortal::<T>::add_proposal(RawOrigin::Signed(relayer.clone()).into(), 1, Proposal {
            _author: CrossChainAccount::Substrate(account("caller", 1, 0)),
            _voting_format: VotingFormat::SingleChoice,
            _option_count: s.try_into().unwrap(),
            _data: IpfsHash::default(),
            _privacy: PrivacyLevel::Mixed,
            _start: 0,
            _end: 7600000,
            _frequency: Some(3600000),
            state: ProposalState::default()
        });
    }: update_vote(RawOrigin::Signed(relayer.clone()), VoteUpdate {
        project: 1,
        proposal: 1,
        votes: vec![0.into(); s.try_into().unwrap()],
        pub_voters: Some(IpfsHash::default())
    })
    verify {
        assert_eq!(DAOPortal::<T>::proposals(1, 1).unwrap().state.updates, 1);
    }
}

impl_benchmark_test_suite!(
    DAOPortal,
    crate::mock::ExtBuilder::default().build(),
    crate::mock::Test,
);
