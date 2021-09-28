use frame_support::{
    assert_noop, assert_ok, 
    traits::{
        Currency, WithdrawReasons, LockIdentifier, LockableCurrency, Imbalance
    }};

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

#[test]
fn check_abnormal_burn() {
    ExtBuilder::default().existential_deposit(100).build().execute_with(|| {
        assert_eq!(Balances::total_issuance(), 100 * 30);
        //we need to use `let imbalance = ` to prevent the PositiveImbalance to be dropped after calling Balance::burn, which will increase the total issuance
        let imbalance = Balances::burn(100 * 25);
        assert_eq!(Balances::total_issuance(), 500);
        assert_ok!(Economics::burn(Some(1).into(), 100 * 7));
        assert_eq!(Balances::total_issuance(), 0);
        assert_eq!(Balances::free_balance(&1), 100 * 5);
    });
}