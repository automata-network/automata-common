use frame_support::{assert_err, assert_noop, assert_ok};
use frame_system::RawOrigin;
use sp_runtime::traits::BadOrigin;

use super::*;
use crate::mock::{Test, Balances, ExtBuilder, Timestamp, Vesting};

const INIT_TIME: u64 = 100;

#[test]
fn check_vesting_status() {
    ExtBuilder::default().existential_deposit(256).build().execute_with(|| {
		assert_eq!(Balances::free_balance(&3), 256 * 30);
        Timestamp::set_timestamp(INIT_TIME);
        assert_eq!(Vesting::vesting_balance(&3), Some(256 * 30));
        Timestamp::set_timestamp(150);
        assert_eq!(Vesting::vesting_balance(&3), Some(256 * 24));
        Timestamp::set_timestamp(151);
        assert_eq!(Vesting::vesting_balance(&3), Some(256 * 24));
        Timestamp::set_timestamp(200);
        assert_eq!(Vesting::vesting_balance(&3), Some(256 * 24));
        Timestamp::set_timestamp(249);
        assert_eq!(Vesting::vesting_balance(&3), Some(256 * 24));
        Timestamp::set_timestamp(250);
        assert_eq!(Vesting::vesting_balance(&3), Some(256 * 16));
        Timestamp::set_timestamp(349);
        assert_eq!(Vesting::vesting_balance(&3), Some(256 * 8));
        Timestamp::set_timestamp(350);
        assert_eq!(Vesting::vesting_balance(&3), Some(0));
    });
}

#[test]
fn check_vested_transfer() {
    ExtBuilder::default().existential_deposit(256).build().execute_with(|| {
        assert_eq!(Balances::free_balance(&3), 256 * 30);
        Timestamp::set_timestamp(350);
        assert_eq!(Vesting::vesting_balance(&3), Some(0));
        let plan = VestingPlan {
            start_time: 400,
            cliff_duration: 0,
            total_duration: 10,
            interval: 10,
            initial_amount: 0,
            total_amount: 256 * 30,
            vesting_during_cliff: false,
        };
        assert_noop!(Vesting::vested_transfer(Some(3).into(), 333, plan), pallet_balances::Error::<Test, _>::LiquidityRestrictions);
        assert_ok!(Vesting::unlock(Some(3).into()));
        assert_ok!(Vesting::vested_transfer(Some(3).into(), 333, plan));
        assert_eq!(Vesting::vesting_balance(&333), Some(256 * 30));
    });
}

#[test]
fn check_unlock() {
    ExtBuilder::default().existential_deposit(256).build().execute_with(|| {
        assert_eq!(Balances::free_balance(&3), 256 * 30);
        Timestamp::set_timestamp(INIT_TIME);
        assert_eq!(Vesting::vesting_balance(&3), Some(256 * 30));
        Timestamp::set_timestamp(250);
        assert_eq!(Vesting::vesting_balance(&3), Some(256 * 16));
        assert_ok!(Vesting::unlock(Some(3).into()));
        //unlocked tokens should be able to transfer
        assert_ok!(Balances::transfer(Some(3).into(), 333, 256 * 13));
        assert_eq!(Balances::free_balance(&3), 256 * 17);
        Timestamp::set_timestamp(349);
        assert_eq!(Vesting::vesting_balance(&3), Some(256 * 8));
        Timestamp::set_timestamp(350);
        assert_eq!(Vesting::vesting_balance(&3), Some(0));
        assert_ok!(Vesting::unlock(Some(3).into()));
        assert_ok!(Balances::transfer(Some(3).into(), 333, 256 * 17));
    });
}

#[test]
fn vesting_exist() {
    ExtBuilder::default().existential_deposit(256).build().execute_with(|| {
        assert_eq!(Balances::free_balance(&3), 256 * 30);
        assert_eq!(Balances::free_balance(&4), 256 * 40);
        Timestamp::set_timestamp(350);
        assert_eq!(Vesting::vesting_balance(&3), Some(0));
        let plan = VestingPlan {
            start_time: 400,
            cliff_duration: 0,
            total_duration: 10,
            interval: 10,
            initial_amount: 0,
            total_amount: 256 * 30,
            vesting_during_cliff: false,
        };
        assert_noop!(Vesting::vested_transfer(Some(4).into(), 3, plan), Error::<Test>::ExistingVestingPlan);
        assert_ok!(Vesting::unlock(Some(3).into()));
        assert_ok!(Vesting::vested_transfer(Some(4).into(), 3, plan));
    });
}

#[test]
fn vested_transfer_amount_low() {
    ExtBuilder::default().existential_deposit(256).build().execute_with(|| {
        Timestamp::set_timestamp(INIT_TIME);
        assert_eq!(Balances::free_balance(&4), 256 * 40);
        let mut plan = VestingPlan {
            start_time: 400,
            cliff_duration: 0,
            total_duration: 10,
            interval: 10,
            initial_amount: 0,
            total_amount: 256 * 1,
            vesting_during_cliff: false,
        };
        assert_noop!(Vesting::vested_transfer(Some(4).into(), 333, plan), Error::<Test>::AmountLow);
        plan.total_amount = 256 * 2;
        assert_ok!(Vesting::vested_transfer(Some(4).into(), 333, plan));
    });
}

#[test]
fn invalid_vesting_param() {
    ExtBuilder::default().existential_deposit(256).build().execute_with(|| {
        Timestamp::set_timestamp(INIT_TIME);
        assert_eq!(Balances::free_balance(&4), 256 * 40);
        let mut plan = VestingPlan {
            start_time: 400,
            cliff_duration: 20,
            total_duration: 10,
            interval: 0,
            initial_amount: 256 * 3,
            total_amount: 256 * 2,
            vesting_during_cliff: false,
        };
        assert_noop!(Vesting::vested_transfer(Some(4).into(), 333, plan), Error::<Test>::InvalidVestingPlan);
        plan.interval = 10;
        assert_noop!(Vesting::vested_transfer(Some(4).into(), 333, plan), Error::<Test>::InvalidVestingPlan);
        plan.total_duration = 20;
        assert_noop!(Vesting::vested_transfer(Some(4).into(), 333, plan), Error::<Test>::InvalidVestingPlan);
        plan.total_amount = 256 * 4;
        assert_ok!(Vesting::vested_transfer(Some(4).into(), 333, plan));
        Timestamp::set_timestamp(419);
        assert_eq!(Vesting::vesting_balance(&333), Some(256 * 1));
        Timestamp::set_timestamp(420);
        assert_eq!(Vesting::vesting_balance(&333), Some(0));
    });
}

#[test]
fn no_vesting() {
    ExtBuilder::default().existential_deposit(256).build().execute_with(|| {
        assert_noop!(Vesting::unlock(Some(345).into()), Error::<Test>::NoVestingPlan);
    });
}