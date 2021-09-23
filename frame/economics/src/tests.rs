use frame_support::{assert_err, assert_noop, assert_ok, traits::{WithdrawReasons, LockIdentifier, LockableCurrency}};
use frame_system::RawOrigin;
use sp_runtime::traits::BadOrigin;

use super::*;
use crate::mock::{Test, Balances, ExtBuilder, Economics};

#[test]
fn check_burn() {
    ExtBuilder::default().existential_deposit(100).build().execute_with(|| {
        assert_eq!(Balances::total_issuance(), 100 * 30);
        assert_ok!(Economics::burn(Some(1).into(), 10));
        assert_eq!(Balances::total_issuance(), 100 * 30 - 10);
        assert_eq!(Balances::free_balance(&1), 100 * 10 - 10);
    });
}

#[test]
fn check_burn_all() {
    ExtBuilder::default().existential_deposit(100).build().execute_with(|| {
        assert_eq!(Balances::total_issuance(), 100 * 30);
        assert_ok!(Economics::burn(Some(1).into(), 100 * 9));
        assert_eq!(Balances::total_issuance(), 100 * 21);
        assert_eq!(Balances::free_balance(&1), 100);
        assert_ok!(Economics::burn(Some(2).into(), 100 * 19));
        assert_eq!(Balances::total_issuance(), 100 * 2);
        assert_eq!(Balances::free_balance(&2), 100);
    });
}

#[test]
fn check_burn_exceed() {
    ExtBuilder::default().existential_deposit(100).build().execute_with(|| {
        assert_eq!(Balances::total_issuance(), 100 * 30);
        assert_noop!(Economics::burn(Some(1).into(), 100 * 10), Error::<Test>::KillAcount);
        assert_eq!(Balances::total_issuance(), 100 * 30);
        assert_eq!(Balances::free_balance(&1), 100 * 10);
    });
}

#[test]
fn check_burn_locked() {
    ExtBuilder::default().existential_deposit(100).build().execute_with(|| {
        const ECO_ID: LockIdentifier = *b"testecon";
        Balances::set_lock(ECO_ID, &1, 100 * 5, WithdrawReasons::TRANSFER);
        assert_eq!(Balances::free_balance(&1), 100 * 10);
        assert_eq!(Balances::total_issuance(), 100 * 30);
        assert_noop!(Economics::burn(Some(1).into(), 501), Error::<Test>::InsufficientLiquidity);
        assert_eq!(Balances::free_balance(&1), 100 * 10);
        assert_eq!(Balances::total_issuance(), 100 * 30);
        assert_ok!(Economics::burn(Some(1).into(), 100 * 5));
        assert_eq!(Balances::free_balance(&1), 100 * 5);
        assert_eq!(Balances::total_issuance(), 100 * 25);
    });
}