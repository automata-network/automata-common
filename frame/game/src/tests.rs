use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;

use super::*;
use crate::mock::{ExtBuilder, Game, System, Test};

#[test]
fn bad_origin() {
    use sp_runtime::DispatchError;

    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(
            Game::attack(Some(1).into(), vec![]),
            DispatchError::BadOrigin
        );
    });
}

#[test]
fn test_maximum_attack_count() {
    ExtBuilder::default().build().execute_with(|| {
        let maximumAttackCount = <Test as Config>::MaximumAttackCount::get();
        let mut i: u64 = 0;
        while i < maximumAttackCount.into() {
            assert_ok!(Game::attack(RawOrigin::Root.into(), vec![1, 2]));
            i = i + 1;
        }
        assert_noop!(
            Game::attack(RawOrigin::Root.into(), vec![5, 6, 7]),
            Error::<Test>::BossDied
        );
    });
}

#[test]
fn test_event() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(Game::attack(RawOrigin::Root.into(), vec![3, 4]));
        System::assert_last_event(crate::Event::AttackBoss(vec![3, 4]).into());
    });
}

#[test]
fn test_attacker_num() {
    ExtBuilder::default().build().execute_with(|| {
        let maximumAttackerNum = <Test as Config>::MaximumAttackerNum::get();
        let mut participates = Vec::<u64>::new();
        let mut i: u64 = 0;
        while i < (maximumAttackerNum + 1).into() {
            participates.push(i);
            i = i + 1;
        }
        assert_noop!(
            Game::attack(RawOrigin::Root.into(), participates),
            Error::<Test>::AttackNumExceed
        );
    });
}
