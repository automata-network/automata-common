use crate as pallet_order;
use crate::mock::*;
use crate::Error;
use crate::{assert_order, assert_service_state, assert_state, create_order, set_order_state};
use automata_traits::order::OrderTrait;
use frame_support::{assert_noop, assert_ok};
use primitives::geodesession::GeodeSessionPhase;
use primitives::order::{OrderOf, OrderState};

fn default_order(provider: u64) -> OrderOf<Test> {
    let order_id = gen_hash(1);
    OrderOf::<Test> {
        order_id,
        binary: "binary".into(),
        encrypted: false,
        domain: "domain".into(),
        name: "name".into(),
        provider,
        price: 0i32.into(),
        start_session_id: 0,
        duration: 1,
        num: 1,
        state: OrderState::Submitted,
        refund_unit: 1,
    }
}

struct BlockManager(u64);
impl BlockManager {
    pub fn new() -> Self {
        System::set_block_number(1);
        BlockManager(1)
    }
    pub fn next_block(&mut self) {
        self.0 = self.0 + 1;
        System::set_block_number(self.0);
    }
}

#[test]
fn test_on_new_session() {
    new_test_ext().execute_with(|| {
        let provider = 1;
        let origin = Origin::signed(provider);
        let mut session = GeodeSession::new();
        {
            // #1: timeout for processing order
            let mut order = default_order(provider);
            order.num = 2;
            let chain_order = create_order!(origin.clone(), order.clone());
            let order_id = chain_order.order_id;
            session.next_phase_to(GeodeSessionPhase::OrderDispatch);
            assert_state!(order_id, OrderState::Pending);
            assert_service_state!(order_id, OrderState::Pending, 2);
            assert_ok!(set_order_state!(0, order_id, OrderState::Processing));
            assert_ok!(set_order_state!(1, order_id, OrderState::Processing));
            assert_service_state!(order_id, OrderState::Processing, 2);
            // wait 2 session to timeout
            session.next_phase_to(GeodeSessionPhase::OrderDispatch);
            assert_state!(order_id, OrderState::Processing);
            session.next_phase_to(GeodeSessionPhase::OrderDispatch);
            assert_state!(order_id, OrderState::Done);
            assert_service_state!(order_id, OrderState::Done, 2);
        }
        {
            // #2: cancel
            <frame_system::Pallet<Test>>::inc_account_nonce(provider);
            let mut order = default_order(provider);
            order.num = 2;
            let chain_order = create_order!(origin.clone(), order.clone());
            let order_id = chain_order.order_id;
            session.next_phase_to(GeodeSessionPhase::OrderDispatch);
            assert_state!(order_id, OrderState::Pending);
            assert_service_state!(order_id, OrderState::Pending, 2);
            assert_ok!(OrderModule::cancel_order(origin, order_id));
            assert_state!(order_id, OrderState::Pending);
            assert_service_state!(order_id, OrderState::Pending, 2);
            session.next_phase_to(GeodeSessionPhase::SessionInitialize);
            assert_state!(order_id, OrderState::Done);
            assert_service_state!(order_id, [OrderState::Done, OrderState::Done]);
        }
    })
}

#[test]
fn it_works_order_state_transition() {
    new_test_ext().execute_with(|| {
        let mut session = GeodeSession::new();
        let provider = 2;
        let origin = Origin::signed(provider);
        let mut order = default_order(provider);
        order.num = 3;
        {
            let chain_order = create_order!(origin.clone(), order.clone());
            let order_id = chain_order.order_id;
            session.next_phase_to(GeodeSessionPhase::OrderDispatch);
            assert_state!(order_id, OrderState::Pending);
            assert_service_state!(order_id, OrderState::Pending, order.num as usize);

            {
                // to processing
                assert_ok!(set_order_state!(0, order_id, OrderState::Processing));
                assert_service_state!(order_id, OrderState::Processing, 1);
                assert_service_state!(order_id, OrderState::Pending, 2);
                assert_state!(order_id, OrderState::Pending);

                assert_ok!(set_order_state!(0, order_id, OrderState::Done));
                assert_service_state!(order_id, OrderState::Done, 1);
                assert_service_state!(order_id, OrderState::Pending, 2);
                assert_state!(order_id, OrderState::Pending);

                assert_ok!(set_order_state!(1, order_id, OrderState::Processing));
                assert_ok!(set_order_state!(2, order_id, OrderState::Processing));
                assert_state!(order_id, OrderState::Processing);
            }
            {
                // to done
                assert_ok!(set_order_state!(1, order_id, OrderState::Done));
                assert_ok!(set_order_state!(2, order_id, OrderState::Done));
                assert_state!(order_id, OrderState::Done);
            }
        }
    })
}

#[test]
fn test_on_orders_dispatch() {
    new_test_ext().execute_with(|| {
        let mut session = GeodeSession::new();
        let provider = 3;
        let origin = Origin::signed(provider);
        {
            // normal case
            let chain_order = create_order!(origin.clone(), default_order(provider));
            let order_id = chain_order.order_id;
            assert_state!(order_id, OrderState::Submitted);
            session.next_phase_to(GeodeSessionPhase::OrderDispatch);
            assert_order!(order_id, start_session_id, session.idx);
            assert_state!(order_id, OrderState::Pending);
            assert_service_state!(order_id, OrderState::Pending, 1);
        }
        {
            // emergency case(available service is not enough)
            <frame_system::Pallet<Test>>::inc_account_nonce(provider);
            let mut order = default_order(provider);
            order.num = 4; // in mock.rs we only dispatch 3 services
            let chain_order = create_order!(origin.clone(), order);
            let order_id = chain_order.order_id;
            assert_state!(order_id, OrderState::Submitted);
            session.next_phase_to(GeodeSessionPhase::OrderDispatch);
            assert_order!(order_id, start_session_id, session.idx);
            assert_state!(order_id, OrderState::Emergency);
            assert_service_state!(order_id, OrderState::Pending, 3);
        }
    })
}

#[test]
fn test_on_emergency_order_dispatch() {
    new_test_ext().execute_with(|| {
        let mut session = GeodeSession::new();
        let provider = 3;
        let origin = Origin::signed(provider);
        {
            // normal case
            let chain_order = create_order!(origin.clone(), default_order(provider));
            let order_id = chain_order.order_id;
            assert_state!(order_id, OrderState::Submitted);
            session.next_phase_to(GeodeSessionPhase::GeodeOffline);
            assert_state!(order_id, OrderState::Submitted);
            session.next_phase_to(GeodeSessionPhase::OrderDispatch);
            assert_state!(order_id, OrderState::Pending);

            assert_ok!(set_order_state!(0, order_id, OrderState::Emergency));
            assert_state!(order_id, OrderState::Emergency);
            session.next_phase_to(GeodeSessionPhase::GeodeOffline);
            assert_state!(order_id, OrderState::Pending);

            assert_ok!(set_order_state!(0, order_id, OrderState::Processing));
            assert_state!(order_id, OrderState::Processing);
        }
        {
            // service not enough
            <frame_system::Pallet<Test>>::inc_account_nonce(provider);
            let mut order = default_order(provider);
            order.duration = 10;
            order.num = 4; // in mock.rs we only dispatch 3 services
            let chain_order = create_order!(origin.clone(), order);
            let order_id = chain_order.order_id;
            assert_state!(order_id, OrderState::Submitted);
            session.next_phase_to(GeodeSessionPhase::OrderDispatch);
            assert_order!(order_id, start_session_id, session.idx);
            assert_state!(order_id, OrderState::Emergency);
            assert_service_state!(order_id, OrderState::Pending, 3);
            session.next_phase_to(GeodeSessionPhase::OrderDispatch);
            assert_service_state!(order_id, OrderState::Pending, 4);
            assert_state!(order_id, OrderState::Pending);
            for _ in 0..4 {
                assert_ok!(set_order_state!(0, order_id, OrderState::Emergency));
            }
            assert_state!(order_id, OrderState::Emergency);
            session.next_phase_to(GeodeSessionPhase::GeodeOffline);
            assert_state!(order_id, OrderState::Emergency);
            assert_service_state!(
                order_id,
                [
                    OrderState::Pending,
                    OrderState::Pending,
                    OrderState::Pending
                ]
            );
        }
    })
}

#[test]
fn test_state_transition() {
    new_test_ext().execute_with(|| {
        let mut block = BlockManager::new();
        let mut session = GeodeSession::new();
        let provider = 3;
        let origin = Origin::signed(provider);
        let order = default_order(provider);
        let chain_order = create_order!(origin.clone(), order.clone());
        let order_id = chain_order.order_id;
        assert_ne!(order.order_id, chain_order.order_id);
        assert_eq!(order.binary, chain_order.binary);
        {
            // normal case
            block.next_block();
            session.next_phase_to(GeodeSessionPhase::OrderDispatch);
            assert!(!<pallet_order::SubmittedOrderIds<Test>>::get(&order_id).is_some());
            assert_state!(order_id, OrderState::Pending);
            assert_service_state!(order_id, OrderState::Pending, order.num as usize);
        }
        {
            // the service notify that it's ready
            block.next_block();
            let services = <pallet_order::OrderServices<Test>>::get(&order_id);
            assert_eq!(services.len(), 1);
            assert_ok!(OrderModule::on_order_state(
                services[0].0,
                order_id,
                OrderState::Processing
            ));
            assert_state!(order_id, OrderState::Processing);
            session.next_phase_to(GeodeSessionPhase::SessionInitialize);
            session.next_phase_to(GeodeSessionPhase::SessionInitialize); // timeout
            assert_state!(order_id, OrderState::Done);
            assert_service_state!(order_id, OrderState::Done, 1);
        }
        {
            // duplicated order_id
            block.next_block();
            assert_noop!(
                OrderModule::create_order(origin.clone(), order.clone()),
                <Error<Test>>::OrderIdDuplicated
            );
            <frame_system::Pallet<Test>>::inc_account_nonce(provider);
            create_order!(origin.clone(), order.clone());
        }
        {
            // emergency dispatch
            <frame_system::Pallet<Test>>::inc_account_nonce(provider);
            let mut order = order.clone();
            order.num = 2;
            let chain_order = create_order!(origin.clone(), order.clone());
            let order_id = chain_order.order_id;
            session.next_phase_to(GeodeSessionPhase::SessionInitialize);
            assert_state!(chain_order.order_id, OrderState::Pending);
            assert_service_state!(chain_order.order_id, OrderState::Pending, 2);

            {
                // both pending
                assert_ok!(set_order_state!(1, order_id, OrderState::Emergency));
                assert_service_state!(order_id, OrderState::Pending, 1);
                assert_state!(order_id, OrderState::Emergency);
                session.next_phase_to(GeodeSessionPhase::GeodeOffline);
                assert_service_state!(order_id, OrderState::Pending, 2);
                assert_state!(order_id, OrderState::Pending);
            }
            {
                // pending and processing
                assert_ok!(set_order_state!(1, order_id, OrderState::Processing));
                assert_service_state!(order_id, [OrderState::Pending, OrderState::Processing]);
                assert_state!(order_id, OrderState::Pending);

                assert_ok!(set_order_state!(0, order_id, OrderState::Emergency));
                assert_service_state!(order_id, [OrderState::Processing]);
                assert_state!(order_id, OrderState::Emergency);

                session.next_phase_to(GeodeSessionPhase::GeodeOffline); // emergency dispatch
                assert_service_state!(order_id, [OrderState::Processing, OrderState::Pending]);
                assert_state!(order_id, OrderState::Pending);
            }
        }
    });
}

#[test]
fn test_on_order_state() {
    new_test_ext().execute_with(|| {
        let mut session = GeodeSession::new();
        let provider = 3;
        let origin = Origin::signed(provider);
        let order = create_order!(origin.clone(), default_order(provider));
        let order_id = order.order_id;
        session.next_phase_to(GeodeSessionPhase::OrderDispatch);
        assert_state!(order_id, OrderState::Pending);
        assert_service_state!(order_id, [OrderState::Pending]);

        // to submitted
        assert_noop!(
            set_order_state!(0, order_id, OrderState::Submitted),
            <Error<Test>>::InvalidState
        );
        assert_noop!(
            set_order_state!(0, order_id, OrderState::Pending),
            <Error<Test>>::InvalidState
        );
        assert_ok!(set_order_state!(0, order_id, OrderState::Processing));
        assert_state!(order_id, OrderState::Processing);

        assert_ok!(set_order_state!(0, order_id, OrderState::Emergency));
        assert_state!(order_id, OrderState::Emergency);

        session.next_phase_to(GeodeSessionPhase::GeodeOffline);
        assert_ok!(set_order_state!(0, order_id, OrderState::Done));
        assert_state!(order_id, OrderState::Done);
    })
}

#[test]
fn test_create_order() {
    new_test_ext().execute_with(|| {
        let provider = 3;
        let mut session = GeodeSession::new();
        let origin = Origin::signed(provider);
        let order = create_order!(origin.clone(), default_order(provider));
        assert_state!(order.order_id, OrderState::Submitted);
        assert!(<pallet_order::SubmittedOrderIds<Test>>::get(&order.order_id).is_some());

        assert_noop!(
            OrderModule::create_order(origin.clone(), default_order(provider)),
            <Error<Test>>::OrderIdDuplicated
        );

        {
            let order = create_order!(Origin::signed(4), default_order(4));
            assert_state!(order.order_id, OrderState::Submitted);

            // it only process 1 order per phase, so we trigger it twice.
            OrderModule::on_orders_dispatch(session.idx);
            OrderModule::on_orders_dispatch(session.idx);
            assert_state!(order.order_id, OrderState::Pending);
            assert_service_state!(order.order_id, OrderState::Pending, 1);
        }

        let origin_order = default_order(provider);
        assert_order!(order.order_id, binary, origin_order.binary);
        assert_order!(order.order_id, provider, provider);
    })
}
