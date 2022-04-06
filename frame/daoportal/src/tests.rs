use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;

use super::*;
use crate::mock::{Balances, DAOPortal, ExtBuilder, System, Test, Timestamp, INIT_BALANCE};

use datastructures::*;

#[test]
fn bad_origin() {
    use sp_runtime::DispatchError;

    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(
            DAOPortal::register_chain(
                Some(2).into(),
                Chain {
                    _protocol: Protocol::Solidity
                }
            ),
            DispatchError::BadOrigin
        );
        assert_noop!(
            DAOPortal::update_relayer(Some(2).into(), 1),
            DispatchError::BadOrigin
        );
        assert_noop!(
            DAOPortal::update_vote_fee(Some(2).into(), 100),
            DispatchError::BadOrigin
        );
    });
}

#[test]
fn direct_project_manipulation() {
    ExtBuilder::default().install().execute_with(|| {
        let valid_workspace = Workspace {
            _chain: 1,
            strategies: vec![Strategy::Solidity(SolidityStrategy::ERC20Balance(
                EthAddress::default(),
            ))],
        };
        // Adding project
        assert_ok!(DAOPortal::add_project(
            Some(2).into(),
            Project {
                usergroup: UserGroup {
                    owner: CrossChainAccount::Substrate(2),
                    admins: Vec::new(),
                    maintainers: Vec::new(),
                    proposers: None,
                },
                data: IpfsHash::default(),
                workspaces: vec![valid_workspace.clone()]
            }
        ));
        // Empty workspace applied to project
        assert_noop!(
            DAOPortal::add_project(
                Some(2).into(),
                Project {
                    usergroup: UserGroup {
                        owner: CrossChainAccount::Substrate(2),
                        admins: Vec::new(),
                        maintainers: Vec::new(),
                        proposers: None,
                    },
                    data: IpfsHash::default(),
                    workspaces: Vec::new()
                }
            ),
            Error::<Test>::InvalidProject
        );
        // Empty strategies applied to workspace
        assert_noop!(
            DAOPortal::add_project(
                Some(2).into(),
                Project {
                    usergroup: UserGroup {
                        owner: CrossChainAccount::Substrate(2),
                        admins: Vec::new(),
                        maintainers: Vec::new(),
                        proposers: None,
                    },
                    data: IpfsHash::default(),
                    workspaces: vec![Workspace {
                        _chain: 1,
                        strategies: Vec::new()
                    }]
                }
            ),
            Error::<Test>::InvalidWorkspace
        );
        // Wrong strategies applied with different framework
        assert_noop!(
            DAOPortal::add_project(
                Some(2).into(),
                Project {
                    usergroup: UserGroup {
                        owner: CrossChainAccount::Substrate(2),
                        admins: Vec::new(),
                        maintainers: Vec::new(),
                        proposers: None,
                    },
                    data: IpfsHash::default(),
                    workspaces: vec![Workspace {
                        _chain: 2,
                        strategies: vec![Strategy::Solidity(SolidityStrategy::ERC20Balance(
                            EthAddress::default()
                        ))]
                    }]
                }
            ),
            Error::<Test>::InvalidStrategy
        );
        assert_noop!(
            DAOPortal::add_project(
                Some(2).into(),
                Project {
                    usergroup: UserGroup {
                        owner: CrossChainAccount::Substrate(2),
                        admins: Vec::new(),
                        maintainers: Vec::new(),
                        proposers: None,
                    },
                    data: IpfsHash::default(),
                    workspaces: vec![valid_workspace.clone(), valid_workspace.clone()]
                }
            ),
            Error::<Test>::DuplicateWorkspace
        );
        assert_eq!(DAOPortal::latest_project_id(), 1);

        // Updating Project
        // changing owner to someone else
        assert_ok!(DAOPortal::update_project(
            Some(2).into(),
            1,
            Project {
                usergroup: UserGroup {
                    owner: CrossChainAccount::Substrate(3),
                    admins: Vec::new(),
                    maintainers: Vec::new(),
                    proposers: None,
                },
                data: IpfsHash::default(),
                workspaces: vec![valid_workspace.clone()]
            }
        ));
        // using same sender after changing owner
        assert_noop!(
            DAOPortal::update_project(
                Some(2).into(),
                1,
                Project {
                    usergroup: UserGroup {
                        owner: CrossChainAccount::Substrate(4),
                        admins: Vec::new(),
                        maintainers: Vec::new(),
                        proposers: None,
                    },
                    data: IpfsHash::default(),
                    workspaces: vec![valid_workspace.clone()]
                }
            ),
            Error::<Test>::InvalidSenderOrigin
        );
        // using changed sender after changing owner
        assert_ok!(DAOPortal::update_project(
            Some(3).into(),
            1,
            Project {
                usergroup: UserGroup {
                    owner: CrossChainAccount::Substrate(4),
                    admins: Vec::new(),
                    maintainers: Vec::new(),
                    proposers: None,
                },
                data: IpfsHash::default(),
                workspaces: vec![valid_workspace]
            }
        ));
    });
}

#[test]
fn relay_project_manipulation() {
    ExtBuilder::default().install().execute_with(|| {
        let valid_workspace = Workspace {
            _chain: 1,
            strategies: vec![Strategy::Solidity(SolidityStrategy::ERC20Balance(
                EthAddress::default(),
            ))],
        };
        // Adding project
        assert_ok!(DAOPortal::add_project(
            Some(1).into(),
            Project {
                usergroup: UserGroup {
                    owner: CrossChainAccount::Solidity(EthAddress::zero()),
                    admins: Vec::new(),
                    maintainers: Vec::new(),
                    proposers: None,
                },
                data: IpfsHash::default(),
                workspaces: vec![valid_workspace.clone()]
            }
        ));
        assert_eq!(DAOPortal::latest_project_id(), 1);

        // Updating project
        // Using a native a user to update
        assert_noop!(
            DAOPortal::update_project(
                Some(2).into(),
                1,
                Project {
                    usergroup: UserGroup {
                        owner: CrossChainAccount::Substrate(2),
                        admins: Vec::new(),
                        maintainers: Vec::new(),
                        proposers: None,
                    },
                    data: IpfsHash::default(),
                    workspaces: vec![valid_workspace.clone()]
                }
            ),
            Error::<Test>::InvalidSenderOrigin
        );
        // Using Relayer to update owner to the native user
        assert_ok!(DAOPortal::update_project(
            Some(1).into(),
            1,
            Project {
                usergroup: UserGroup {
                    owner: CrossChainAccount::Substrate(2),
                    admins: Vec::new(),
                    maintainers: Vec::new(),
                    proposers: None,
                },
                data: IpfsHash::default(),
                workspaces: vec![valid_workspace.clone()]
            }
        ));
        // Using the native a user to update again
        assert_ok!(DAOPortal::update_project(
            Some(2).into(),
            1,
            Project {
                usergroup: UserGroup {
                    owner: CrossChainAccount::Substrate(3),
                    admins: Vec::new(),
                    maintainers: Vec::new(),
                    proposers: None,
                },
                data: IpfsHash::default(),
                workspaces: vec![valid_workspace.clone()]
            }
        ));
    });
}

#[test]
fn direct_add_proposal() {
    ExtBuilder::default().install_w_project().execute_with(|| {
        // Add a valid proposal
        assert_ok!(DAOPortal::add_proposal(
            Some(2).into(),
            1,
            DAOProposal {
                _author: CrossChainAccount::Substrate(2),
                _voting_format: VotingFormat::SingleChoice,
                _option_count: 2,
                _data: IpfsHash::default(),
                _privacy: PrivacyLevel::Opaque(1),
                _start: 2000,
                _end: 5000,
                _frequency: None,
                _workspaces: Vec::new(),
                state: DAOProposalState::default()
            }
        ));
        assert_eq!(DAOPortal::latest_proposal_id(1), 1);
        let proposal_1 = DAOPortal::proposals(1, 1).unwrap();
        assert_eq!(
            Balances::free_balance(&2),
            INIT_BALANCE - DAOPortal::vote_fee()
        );
        assert_eq!(
            Balances::free_balance(&1),
            INIT_BALANCE + DAOPortal::vote_fee()
        );

        // Add another valid proposal with 0 pending period
        assert_ok!(DAOPortal::add_proposal(
            Some(2).into(),
            1,
            DAOProposal {
                _author: CrossChainAccount::Substrate(2),
                _voting_format: VotingFormat::SingleChoice,
                _option_count: 2,
                _data: IpfsHash::default(),
                _privacy: PrivacyLevel::Opaque(1),
                _start: 1000,
                _end: 5000,
                _frequency: None,
                _workspaces: Vec::new(),
                state: DAOProposalState::default()
            }
        ));

        assert_eq!(DAOPortal::latest_proposal_id(1), 2);
        let proposal_2 = DAOPortal::proposals(1, 2).unwrap();

        // Add an invalid proposal with invalid number of options
        assert_noop!(
            DAOPortal::add_proposal(
                Some(2).into(),
                1,
                DAOProposal {
                    _author: CrossChainAccount::Substrate(2),
                    _voting_format: VotingFormat::SingleChoice,
                    _option_count: 1,
                    _data: IpfsHash::default(),
                    _privacy: PrivacyLevel::Opaque(1),
                    _start: 2000,
                    _end: 5000,
                    _frequency: None,
                    _workspaces: Vec::new(),
                    state: DAOProposalState::default()
                }
            ),
            Error::<Test>::InvalidProposal
        );

        // Add an invalid proposal with invalid number of options
        assert_noop!(
            DAOPortal::add_proposal(
                Some(2).into(),
                1,
                DAOProposal {
                    _author: CrossChainAccount::Substrate(2),
                    _voting_format: VotingFormat::SingleChoice,
                    _option_count: 4,
                    _data: IpfsHash::default(),
                    _privacy: PrivacyLevel::Opaque(1),
                    _start: 2000,
                    _end: 5000,
                    _frequency: None,
                    _workspaces: Vec::new(),
                    state: DAOProposalState::default()
                }
            ),
            Error::<Test>::InvalidProposal
        );

        // Add an invalid proposal with invalid duration
        assert_noop!(
            DAOPortal::add_proposal(
                Some(2).into(),
                1,
                DAOProposal {
                    _author: CrossChainAccount::Substrate(2),
                    _voting_format: VotingFormat::SingleChoice,
                    _option_count: 2,
                    _data: IpfsHash::default(),
                    _privacy: PrivacyLevel::Opaque(1),
                    _start: 2000,
                    _end: 2500,
                    _frequency: None,
                    _workspaces: Vec::new(),
                    state: DAOProposalState::default()
                }
            ),
            Error::<Test>::InvalidDuration
        );

        // Add an invalid proposal with invalid duration
        assert_noop!(
            DAOPortal::add_proposal(
                Some(2).into(),
                1,
                DAOProposal {
                    _author: CrossChainAccount::Substrate(2),
                    _voting_format: VotingFormat::SingleChoice,
                    _option_count: 2,
                    _data: IpfsHash::default(),
                    _privacy: PrivacyLevel::Opaque(1),
                    _start: 2000,
                    _end: 12001,
                    _frequency: None,
                    _workspaces: Vec::new(),
                    state: DAOProposalState::default()
                }
            ),
            Error::<Test>::InvalidDuration
        );

        // Add a proposal from account with 0 balance
        assert_noop!(
            DAOPortal::add_proposal(
                Some(3).into(),
                1,
                DAOProposal {
                    _author: CrossChainAccount::Substrate(2),
                    _voting_format: VotingFormat::SingleChoice,
                    _option_count: 2,
                    _data: IpfsHash::default(),
                    _privacy: PrivacyLevel::Opaque(1),
                    _start: 2000,
                    _end: 5000,
                    _frequency: None,
                    _workspaces: Vec::new(),
                    state: DAOProposalState::default()
                }
            ),
            Error::<Test>::InsufficientBalance
        );

        assert_ok!(Balances::transfer(Some(2).into(), 3, DAOPortal::vote_fee()));
        assert_eq!(Balances::free_balance(&3), DAOPortal::vote_fee());

        // Add a proposal from account with balance for paying fee for only one update
        assert_noop!(
            DAOPortal::add_proposal(
                Some(3).into(),
                1,
                DAOProposal {
                    _author: CrossChainAccount::Substrate(2),
                    _voting_format: VotingFormat::SingleChoice,
                    _option_count: 2,
                    _data: IpfsHash::default(),
                    _privacy: PrivacyLevel::Private,
                    _start: 2000,
                    _end: 5000,
                    _frequency: Some(1000),
                    _workspaces: Vec::new(),
                    state: DAOProposalState::default()
                }
            ),
            Error::<Test>::InsufficientBalance
        );

        assert_ok!(Balances::transfer(
            Some(2).into(),
            3,
            2 * DAOPortal::vote_fee()
        ));
        assert_eq!(Balances::free_balance(&3), 3 * DAOPortal::vote_fee());

        // Add a proposal from account with sufficient balance for paying udpate fee
        assert_ok!(DAOPortal::add_proposal(
            Some(3).into(),
            1,
            DAOProposal {
                _author: CrossChainAccount::Substrate(2),
                _voting_format: VotingFormat::SingleChoice,
                _option_count: 2,
                _data: IpfsHash::default(),
                _privacy: PrivacyLevel::Private,
                _start: 2000,
                _end: 5000,
                _frequency: Some(1000),
                _workspaces: Vec::new(),
                state: DAOProposalState::default()
            }
        ));

        // Ensure that 3 update fees were costed
        assert_eq!(Balances::free_balance(&3), 0);
    });
}

#[test]
fn relay_add_proposal() {
    ExtBuilder::default().install_w_project().execute_with(|| {
        // Add a valid proposal from relayer
        assert_ok!(DAOPortal::add_proposal(
            Some(1).into(),
            1,
            DAOProposal {
                _author: CrossChainAccount::Substrate(2),
                _voting_format: VotingFormat::SingleChoice,
                _option_count: 2,
                _data: IpfsHash::default(),
                _privacy: PrivacyLevel::Private,
                _start: 2000,
                _end: 5000,
                _frequency: Some(1),
                _workspaces: Vec::new(),
                state: DAOProposalState::default()
            }
        ));

        // no vote fee costed
        assert_eq!(Balances::free_balance(&1), INIT_BALANCE);
    });
}

#[test]
fn update_vote() {
    ExtBuilder::default().install_w_proposal().execute_with(|| {
        // Still pending
        assert_noop!(
            DAOPortal::update_vote(
                Some(1).into(),
                VoteUpdate {
                    project: 1,
                    proposal: 1,
                    votes: vec![0.into(); 2],
                    pub_voters: None,
                }
            ),
            Error::<Test>::InvalidStatus
        );

        Timestamp::set_timestamp(2000);

        // Proposal (1, 1) voted failed with incorrect votes size
        assert_noop!(
            DAOPortal::update_vote(
                Some(1).into(),
                VoteUpdate {
                    project: 1,
                    proposal: 1,
                    votes: vec![0.into(); 3],
                    pub_voters: None,
                }
            ),
            Error::<Test>::InvalidVote
        );
        // Proposal (1, 1) vote okay
        assert_ok!(DAOPortal::update_vote(
            Some(1).into(),
            VoteUpdate {
                project: 1,
                proposal: 1,
                votes: vec![0.into(); 2],
                pub_voters: None
            }
        ));

        // Proposal (1, 2) voted failed with pub_voters not None
        assert_noop!(
            DAOPortal::update_vote(
                Some(1).into(),
                VoteUpdate {
                    project: 1,
                    proposal: 2,
                    votes: vec![0.into(); 2],
                    pub_voters: Some(IpfsHash::default()),
                }
            ),
            Error::<Test>::ConflictWithPrivacyLevel
        );

        Timestamp::set_timestamp(5000);

        // Proposal (1, 3) voted okay
        assert_ok!(DAOPortal::update_vote(
            Some(1).into(),
            VoteUpdate {
                project: 1,
                proposal: 3,
                votes: vec![0.into(); 2],
                pub_voters: None,
            }
        ));
        assert_eq!(DAOPortal::proposals(1, 3).unwrap().state.finalized, true);
    });
}
