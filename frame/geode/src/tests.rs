use crate as pallet_geode;
use crate::{mock::*, Error};
use automata_traits::{attestor::ApplicationTrait, geode::GeodeTrait};
use frame_support::{assert_noop, assert_ok};
use pallet_geode::WorkingState;

#[test]
fn it_works_register_geode() {
    new_test_ext().execute_with(|| {
        let attestor_account = 1;
        let geode_account = 2;
        let geode_id = 3;
        let provider = 4;

        register_attestor(attestor_account);

        let geode: pallet_geode::Geode<
            <Test as frame_system::Config>::AccountId,
            <Test as frame_system::Config>::Hash,
            <Test as frame_system::Config>::BlockNumber,
        > = pallet_geode::Geode {
            id: geode_id,
            provider: provider,
            order_id: None,
            ip: vec![],
            domain: vec![],
            props: Default::default(),
            working_state: Default::default(),
            healthy_state: Default::default(),
        };

        assert_ok!(GeodeModule::register_geode(Origin::signed(provider), geode));
    });
}

#[test]
fn it_works_geode_remove() {
    new_test_ext().execute_with(|| {
        let attestor_account = 1;
        let geode_account = 2;
        let geode_id = 3;
        let provider = 4;

        register_attestor(attestor_account);

        let geode: pallet_geode::Geode<
            <Test as frame_system::Config>::AccountId,
            <Test as frame_system::Config>::Hash,
            <Test as frame_system::Config>::BlockNumber,
        > = pallet_geode::Geode {
            id: geode_id,
            provider: provider,
            order_id: None,
            ip: vec![],
            domain: vec![],
            props: Default::default(),
            working_state: Default::default(),
            healthy_state: Default::default(),
        };

        assert_ok!(GeodeModule::register_geode(Origin::signed(provider), geode));

        assert_ok!(GeodeModule::remove_geode(
            Origin::signed(provider),
            geode_id
        ));
    });
}

#[test]
fn it_works_update_geode_props() {
    new_test_ext().execute_with(|| {
        let attestor_account = 1;
        let geode_account = 2;
        let geode_id = 3;
        let provider = 4;

        let origin = Origin::signed(provider);

        assert_ok!(register_attestor(attestor_account));

        let geode = pallet_geode::Geode {
            id: geode_id,
            provider,
            order_id: None,
            ip: vec![],
            domain: vec![],
            props: Default::default(),
            working_state: Default::default(),
            healthy_state: Default::default(),
        };

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
        let attestor_account = 1;
        let geode_account = 2;
        let geode_id = 3;
        let provider = 4;

        assert_ok!(register_attestor(attestor_account));

        let geode = pallet_geode::Geode {
            id: geode_id,
            provider,
            order_id: None,
            ip: vec![],
            domain: vec![],
            props: Default::default(),
            working_state: Default::default(),
            healthy_state: Default::default(),
        };

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

#[test]
fn it_works_states() {
    // idle, pending, working, finalizing, exiting, exited
    new_test_ext().execute_with(|| {
        let attestor_account = 1;
        let geode_account = 2;
        let geode_id = 3;
        let provider = 4;
        let block_number = 1;

        let origin = Origin::signed(provider);
        assert_ok!(GeodeModule::register_geode(
            origin.clone(),
            pallet_geode::Geode {
                id: geode_id,
                provider,
                order_id: None,
                ip: vec![],
                domain: vec![],
                props: Default::default(),
                working_state: Default::default(),
                healthy_state: Default::default(),
            }
        ));

        let session_index = 1;
        let order_id = gen_hash(1);

        // idle -> pending
        {
            assert_noop!(
                GeodeModule::on_order_dispatched(geode_id, session_index, order_id),
                Error::<Test>::GeodeNotHealthy,
            );

            assert_ok!(GeodeModule::application_healthy(geode_id));
            assert_ok!(GeodeModule::on_order_dispatched(
                geode_id,
                session_index,
                order_id
            ));
            let geode = GeodeModule::geodes(geode_id).unwrap();
            assert_eq!(geode.working_state, WorkingState::Pending { session_index });
        }

        // pending -> working
        {
            assert_ok!(GeodeModule::geode_ready(Origin::signed(geode_id), order_id));
            let geode = GeodeModule::geodes(geode_id).unwrap();
            assert_eq!(
                geode.working_state,
                WorkingState::Working {
                    session_index,
                    block_number
                }
            );
        }

        // working -> finalizing
        {
            assert_ok!(GeodeModule::geode_finalizing(
                Origin::signed(geode_id),
                order_id
            ));
            let geode = GeodeModule::geodes(geode_id).unwrap();
            assert_eq!(
                geode.working_state,
                WorkingState::Finalizing { session_index }
            );
        }

        // finalizing -> idle
        {
            assert_ok!(GeodeModule::geode_finalized(
                Origin::signed(geode_id),
                order_id
            ));
            let geode = GeodeModule::geodes(geode_id).unwrap();
            assert_eq!(geode.working_state, WorkingState::Idle);
        }
    });
}
