use crate as pallet_geode;
use crate::{assert_geode, assert_state};
use crate::{mock::*, Error};
use automata_traits::{attestor::ApplicationTrait, geode::GeodeTrait};
use frame_support::{assert_noop, assert_ok};
use pallet_geode::WorkingState;
use primitives::geodesession::GeodeSessionPhase;
use sp_core::H256;

#[test]
fn it_works_register_geode() {
    new_test_ext().execute_with(|| {
        let geode_id = 1;
        let provider = 2;

        let geode = pallet_geode::Geode {
            id: geode_id,
            provider,
            order_id: Some(H256::default()),
            ip: vec![],
            domain: vec![],
            props: Default::default(),
            working_state: pallet_geode::WorkingState::Finalizing { session_index: 1 },
            healthy_state: pallet_geode::HealthyState::Healthy,
        };

        assert_ok!(GeodeModule::register_geode(Origin::signed(provider), geode));

        assert_eq!(
            <pallet_geode::Geodes<Test>>::get(&geode_id),
            Some(pallet_geode::GeodeOf::<Test> {
                id: geode_id,
                provider,
                order_id: None,
                ip: vec![],
                domain: vec![],
                props: Default::default(),
                working_state: pallet_geode::WorkingState::Idle,
                healthy_state: pallet_geode::HealthyState::Unhealthy,
            })
        );
    });
}

#[test]
fn it_works_geode_remove() {
    new_test_ext().execute_with(|| {
        let geode_id = 1;
        let provider = 2;
        let mut session = GeodeSession::new();
        let geode = default_geode(provider, geode_id);
        {
            // testcase #1: it's idle, so remove it directly.
            assert_ok!(GeodeModule::register_geode(
                Origin::signed(provider),
                geode.clone()
            ));
            assert_noop!(
                GeodeModule::remove_geode(Origin::signed(geode_id), geode_id),
                <Error<Test>>::NotOwner
            );
            assert_ok!(GeodeModule::remove_geode(
                Origin::signed(provider),
                geode_id
            ));

            // its state is idle, so it goes exiting directly
            assert!(<pallet_geode::Geodes<Test>>::get(&geode_id).is_some());
            assert!(<pallet_geode::OfflineRequests<Test>>::get(&geode_id).is_none());
            session.next_phase_to(GeodeSessionPhase::SessionInitialize);
            assert!(<pallet_geode::Geodes<Test>>::get(&geode_id).is_none());
        }
        {
            // testcase #2: it's on other state
            assert_ok!(GeodeModule::register_geode(
                Origin::signed(provider),
                geode.clone()
            ));
            assert_ok!(GeodeModule::application_healthy(geode_id));

            // dispatch order to this geode
            let order_id = H256::default();
            session.next_phase_to(GeodeSessionPhase::OrderDispatch);
            GeodeModule::on_order_dispatched(session.idx, order_id, 1, "domain".into());
            assert_state!(
                geode_id,
                WorkingState::Pending {
                    session_index: session.idx
                }
            );
            assert_ok!(GeodeModule::remove_geode(
                Origin::signed(provider),
                geode_id
            ));

            // chain geode state should not changed
            assert_state!(
                geode_id,
                WorkingState::Pending {
                    session_index: session.idx
                }
            );

            assert!(<pallet_geode::OfflineRequests<Test>>::get(&geode_id).is_some());

            // to working
            assert_ok!(GeodeModule::geode_ready(Origin::signed(geode_id), order_id));
            // to finalizing
            assert_ok!(GeodeModule::geode_finalizing(
                Origin::signed(geode_id),
                order_id
            ));
            // to idle
            assert_ok!(GeodeModule::geode_finalized(
                Origin::signed(geode_id),
                order_id
            ));
            session.next_phase_to(GeodeSessionPhase::GeodeOffline);
            assert!(<pallet_geode::OfflineRequests<Test>>::get(&geode_id).is_none());
            assert_state!(geode_id, WorkingState::Exiting);
            session.next_phase_to(GeodeSessionPhase::SessionInitialize);
            assert!(<pallet_geode::Geodes<Test>>::get(&geode_id).is_none());
        }
    });
}

#[test]
fn it_works_update_geode_props() {
    new_test_ext().execute_with(|| {
        let geode_id = 1;
        let provider = 2;
        let origin = Origin::signed(provider);
        let geode = default_geode(provider, geode_id);
        assert_ok!(GeodeModule::register_geode(origin.clone(), geode));
        let prop_name = vec![0x79_u8, 0x70_u8];
        let prop_value = vec![1_u8];
        assert_ok!(GeodeModule::update_geode_props(
            origin.clone(),
            geode_id,
            prop_name.clone(),
            prop_value.clone()
        ));
        let result = GeodeModule::geodes(geode_id).unwrap();
        assert_eq!(result.props.get(&prop_name), Some(&prop_value));
    });
}

#[test]
fn it_works_update_geode_domain() {
    new_test_ext().execute_with(|| {
        let geode_id = 3;
        let provider = 4;

        let geode = default_geode(provider, geode_id);
        assert_ok!(GeodeModule::register_geode(Origin::signed(provider), geode));

        let domain = vec![1_u8];
        assert_ok!(GeodeModule::update_geode_domain(
            Origin::signed(provider),
            geode_id,
            domain.clone()
        ));

        let result = GeodeModule::geodes(geode_id);
        assert_eq!(result.unwrap().domain, domain);
    });
}

fn default_geode(provider: AccountId, id: AccountId) -> pallet_geode::GeodeOf<Test> {
    pallet_geode::GeodeOf::<Test> {
        id,
        provider,
        order_id: None,
        ip: vec![],
        domain: vec![],
        props: Default::default(),
        working_state: Default::default(),
        healthy_state: Default::default(),
    }
}

#[test]
fn it_works_order_dispatch() {
    new_test_ext().execute_with(|| {
        let provider = 0;
        let mut session = GeodeSession::new();

        let origin = Origin::signed(provider);
        let geode1 = default_geode(provider, 1);
        assert_ok!(GeodeModule::register_geode(origin.clone(), geode1.clone()));
        let geode2 = default_geode(provider, 2);
        assert_ok!(GeodeModule::register_geode(origin.clone(), geode2.clone()));
        let geode3 = default_geode(provider, 3);
        assert_ok!(GeodeModule::register_geode(origin.clone(), geode3.clone()));
        let order_id = gen_hash(1);
        let order_domain: Vec<u8> = "order-1".into();
        {
            // #1: unhealth geode should not be dispatched.

            // only geode3 is healthy
            assert_ok!(GeodeModule::application_healthy(geode3.id));

            session.next_phase_to(GeodeSessionPhase::OrderDispatch);
            GeodeModule::on_order_dispatched(session.idx, order_id, 3, order_domain.clone());
            assert_state!(geode1.id, WorkingState::Idle);
            assert_geode!(geode1.id, order_id, None);
            assert_state!(geode2.id, WorkingState::Idle);
            assert_geode!(geode2.id, order_id, None);
            assert_state!(
                geode3.id,
                WorkingState::Pending {
                    session_index: session.idx
                }
            );
            assert_geode!(geode3.id, order_id, Some(order_id));
            assert_geode!(geode3.id, domain, order_domain);
        }
        {
            // #2: geodes which already have OfflineRequest should not be dispatched;
            session.next_phase_to(GeodeSessionPhase::GeodeOffline);
            assert_ok!(GeodeModule::remove_geode(origin.clone(), geode3.id));
            let origin3 = Origin::signed(geode3.id);
            assert_ok!(GeodeModule::geode_ready(origin3.clone(), order_id)); // to working
            assert_ok!(GeodeModule::geode_finalizing(origin3.clone(), order_id)); // to finalizing
            assert_ok!(GeodeModule::geode_finalized(origin3.clone(), order_id)); // to idle
            assert_state!(geode3.id, WorkingState::Idle);
            session.next_phase_to(GeodeSessionPhase::OrderDispatch);
            assert_state!(geode3.id, WorkingState::Idle);
            session.next_phase_to(GeodeSessionPhase::GeodeOffline);
            session.next_phase_to(GeodeSessionPhase::SessionInitialize);
            assert!(<pallet_geode::Geodes<Test>>::get(geode3.id).is_none());
        }
    })
}
