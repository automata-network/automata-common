use crate::mock::{ExtBuilder, Gmetadata, System, Test};
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;

use super::datastructures::*;
use super::*;

#[test]
fn bad_origin() {
    use sp_runtime::DispatchError;

    ExtBuilder::default().build().execute_with(|| {
        let u1 = Some(1);
        let u2 = Some(2);
        assert_noop!(
            Gmetadata::create_namespace(u1.into(), vec![]),
            DispatchError::BadOrigin
        );
        assert_ok!(Gmetadata::create_namespace(
            RawOrigin::Root.into(),
            "ns1".into()
        ));

        assert_noop!(
            Gmetadata::namespace_add_owner(u1.into(), 1, u1.unwrap()),
            DispatchError::BadOrigin
        );
        assert_ok!(Gmetadata::namespace_add_owner(
            RawOrigin::Root.into(),
            1,
            u1.unwrap()
        ));

        assert_noop!(
            Gmetadata::namespace_remove_owner(u2.into(), 1, u1.unwrap()),
            DispatchError::BadOrigin
        );
        assert_ok!(Gmetadata::namespace_remove_owner(u1.into(), 1, u1.unwrap()));
        assert_noop!(
            Gmetadata::namespace_add_owner(u1.into(), 1, u1.unwrap()),
            DispatchError::BadOrigin
        );
        assert_ok!(Gmetadata::namespace_add_owner(
            RawOrigin::Root.into(),
            1,
            u1.unwrap()
        ));

        let key = GmetadataKey {
            ns: 1,
            table: "t1".into(),
            pk: "".into(),
        };
        assert_noop!(
            Gmetadata::set_value(u2.into(), key.clone(), "1".into()),
            DispatchError::BadOrigin
        );
        assert_ok!(Gmetadata::set_value(u1.into(), key.clone(), "1".into()));
        assert_noop!(
            Gmetadata::remove_value(u2.into(), key.clone()),
            DispatchError::BadOrigin
        );
        assert_ok!(Gmetadata::remove_value(u1.into(), key.clone()));

        assert_noop!(
            Gmetadata::add_index(u2.into(), key.clone(), "1".into()),
            DispatchError::BadOrigin
        );
        assert_ok!(Gmetadata::add_index(u1.into(), key.clone(), "1".into()));
        assert_noop!(
            Gmetadata::remove_index(u2.into(), key.clone(), "1".into()),
            DispatchError::BadOrigin
        );
        assert_ok!(Gmetadata::remove_index(u1.into(), key.clone(), "1".into()));

    });
}

#[test]
fn test_namespace() {
    use sp_runtime::DispatchError;
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(
            Gmetadata::create_namespace(Some(1).into(), vec![]),
            DispatchError::BadOrigin
        );

        assert_noop!(
            Gmetadata::create_namespace(RawOrigin::Root.into(), vec![]),
            Error::<Test>::InvalidNamespaceName,
        );

        assert_ok!(Gmetadata::create_namespace(
            RawOrigin::Root.into(),
            "ns1".into()
        ));

        assert_eq!(Gmetadata::get_namespace_id("ns1".as_bytes()), Some(1));

        assert_noop!(
            Gmetadata::create_namespace(RawOrigin::Root.into(), "ns1".into()),
            Error::<Test>::NamespaceAlreadyExist
        );

        assert_ok!(Gmetadata::create_namespace(
            RawOrigin::Root.into(),
            "ns2".into()
        ));

        assert_eq!(Gmetadata::get_namespace_id("ns2".as_bytes()), Some(2));
    });
}

#[test]
fn test_value() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(Gmetadata::create_namespace(
            RawOrigin::Root.into(),
            "ns1".into()
        ));
        let mut key = GmetadataKey {
            ns: 0,
            table: "network".into(),
            pk: "1".into(),
        };
        let value = r#"{"id":"1"}"#.as_bytes();
        let u1 = Some(1);
        assert_noop!(
            Gmetadata::set_value(u1.into(), key.clone(), r#"{"id":"1"}"#.into()),
            Error::<Test>::NamespaceNotFound
        );
        key.ns = 1;
        {
            // owner check
        }
        assert_ok!(Gmetadata::namespace_add_owner(
            RawOrigin::Root.into(),
            1,
            u1.unwrap()
        ));
        assert_ok!(Gmetadata::set_value(u1.into(), key.clone(), value.into()));
        assert_eq!(
            Gmetadata::get_value(key.clone()),
            Some(GmetadataValueInfo {
                data: value.into(),
                update_time: 0,
            })
        );
        assert_ok!(Gmetadata::remove_value(u1.into(), key.clone()));
        assert_eq!(Gmetadata::get_value(key.clone()), None);
    });
}

#[test]
fn test_index() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(Gmetadata::create_namespace(
            RawOrigin::Root.into(),
            "ns1".into()
        ));
        let u1 = Some(1);
        let mut key = GmetadataKey {
            ns: 0,
            table: "network".into(),
            pk: "".into(),
        };
        assert_noop!(
            Gmetadata::add_index(u1.into(), key.clone(), "1".into()),
            Error::<Test>::NamespaceNotFound
        );
        key.ns = 1;
        assert_ok!(Gmetadata::namespace_add_owner(
            RawOrigin::Root.into(),
            1,
            u1.unwrap()
        ));
        assert_ok!(Gmetadata::add_index(u1.into(), key.clone(), "1".into()));
        assert_ok!(Gmetadata::add_index(u1.into(), key.clone(), "3".into()));
        assert_ok!(Gmetadata::add_index(u1.into(), key.clone(), "2".into()));
        assert_eq!(
            Gmetadata::get_index(key.clone()),
            Some(GmetadataIndexInfo {
                data: vec!["1".into(), "2".into(), "3".into()],
                update_time: 0,
            })
        );
        key.ns = 0;
        assert_noop!(
            Gmetadata::remove_index(u1.into(), key.clone(), "1".into()),
            Error::<Test>::NamespaceNotFound
        );
        key.ns = 1;
        assert_ok!(Gmetadata::remove_index(u1.into(), key.clone(), "1".into()));
        assert_eq!(
            Gmetadata::get_index(key.clone()),
            Some(GmetadataIndexInfo {
                data: vec!["2".into(), "3".into()],
                update_time: 0,
            })
        );
    })
}

#[test]
fn test_query() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(Gmetadata::create_namespace(
            RawOrigin::Root.into(),
            "ns1".into()
        ));
        let u1 = Some(1);
        assert_ok!(Gmetadata::namespace_add_owner(
            RawOrigin::Root.into(),
            1,
            u1.unwrap()
        ));
        let mut key = GmetadataKey {
            ns: 1,
            table: "network".into(),
            pk: "".into(),
        };
        assert_ok!(Gmetadata::add_index(u1.into(), key.clone(), "1".into()));
        assert_ok!(Gmetadata::add_index(u1.into(), key.clone(), "2".into()));
        key.pk = "1".into();
        assert_ok!(Gmetadata::set_value(
            u1.into(),
            key.clone(),
            "network1".into()
        ));
        key.pk = "2".into();
        assert_ok!(Gmetadata::set_value(
            u1.into(),
            key.clone(),
            "network2".into()
        ));
        key.pk = "".into();
        assert_eq!(
            Gmetadata::query_with_index(key.clone(), key.clone(), None, 10),
            GmetadataQueryResult{
                list: vec!["network1".into(), "network2".into()],
                cursor: "".into(),
            }
        );
        assert_eq!(
            Gmetadata::query_with_index(key.clone(), key.clone(), None, 1),
            GmetadataQueryResult{
                list: vec!["network1".into()],
                cursor: "1".into(),
            }
        );
        assert_eq!(
            Gmetadata::query_with_index(key.clone(), key.clone(), Some("1".into()), 1),
            GmetadataQueryResult{
                list: vec!["network2".into()],
                cursor: "".into(),
            }
        );
    })
}
