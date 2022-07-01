use crate::mock::{ExtBuilder, Gmetadata, System, Test};
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;
use sp_core::H256;

use super::datastructures::*;
use super::*;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[test]
fn test_query_with_index() {
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    #[derive(codec::Encode)]
    struct Arg {
        index_key: GmetadataKey,
        value_key: GmetadataKey,
        cursor: HexBytes,
        limit: u64,
    }

    let arg = Arg {
        index_key: GmetadataKey {
            ns: 1,
            table: "network".into(),
            pk: "".into(),
        },
        value_key: GmetadataKey {
            ns: 1,
            table: "network".into(),
            pk: "".into(),
        },
        cursor: "".into(),
        limit: 10,
    };
    let value = serde_json::value::Value::Array(vec![
        serde_json::to_value(&arg.index_key).unwrap(),
        serde_json::to_value(&arg.value_key).unwrap(),
        serde_json::to_value(&arg.cursor).unwrap(),
        serde_json::to_value(&arg.limit).unwrap(),
    ]);
    let expect_req = r#"[{"ns":1,"pk":"0x","table":"0x6e6574776f726b"},{"ns":1,"pk":"0x","table":"0x6e6574776f726b"},"0x",10]"#;
    assert_eq!(serde_json::to_string(&value).unwrap(), expect_req.to_string());
}

#[test]
fn bad_origin() {
    use sp_runtime::DispatchError;

    ExtBuilder::default().build().execute_with(|| {
        let mut req_id = H256::default();
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
            Gmetadata::set_value(u2.into(), key.clone(), "1".into(), req_id),
            DispatchError::BadOrigin
        );
        assert_ok!(Gmetadata::set_value(
            u1.into(),
            key.clone(),
            "1".into(),
            req_id
        ));
        assert_noop!(
            Gmetadata::remove_value(u2.into(), key.clone(), req_id),
            DispatchError::BadOrigin
        );
        assert_ok!(Gmetadata::remove_value(u1.into(), key.clone(), req_id));

        assert_noop!(
            Gmetadata::add_index(u2.into(), key.clone(), "1".into(), req_id),
            DispatchError::BadOrigin
        );
        assert_ok!(Gmetadata::add_index(
            u1.into(),
            key.clone(),
            "1".into(),
            req_id
        ));
        assert_noop!(
            Gmetadata::remove_index(u2.into(), key.clone(), "1".into(), req_id),
            DispatchError::BadOrigin
        );
        assert_ok!(Gmetadata::remove_index(
            u1.into(),
            key.clone(),
            "1".into(),
            req_id
        ));
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
        let mut req_id = H256::default();
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
            Gmetadata::set_value(u1.into(), key.clone(), r#"{"id":"1"}"#.into(), req_id),
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
        assert_ok!(Gmetadata::set_value(
            u1.into(),
            key.clone(),
            value.into(),
            req_id
        ));
        assert_eq!(
            Gmetadata::get_value(key.clone()),
            Some(GmetadataValueInfo {
                data: value.into(),
                update_time: 0,
            })
        );
        assert_ok!(Gmetadata::remove_value(u1.into(), key.clone(), req_id));
        assert_eq!(Gmetadata::get_value(key.clone()), None);
    });
}

#[test]
fn test_index() {
    ExtBuilder::default().build().execute_with(|| {
        let mut req_id = H256::default();

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
            Gmetadata::add_index(u1.into(), key.clone(), "1".into(), req_id),
            Error::<Test>::NamespaceNotFound
        );
        key.ns = 1;
        assert_ok!(Gmetadata::namespace_add_owner(
            RawOrigin::Root.into(),
            1,
            u1.unwrap()
        ));
        assert_ok!(Gmetadata::add_index(
            u1.into(),
            key.clone(),
            "1".into(),
            req_id
        ));
        assert_ok!(Gmetadata::add_index(
            u1.into(),
            key.clone(),
            "3".into(),
            req_id
        ));
        assert_ok!(Gmetadata::add_index(
            u1.into(),
            key.clone(),
            "2".into(),
            req_id
        ));
        assert_eq!(
            Gmetadata::get_index(key.clone()),
            Some(GmetadataIndexInfo {
                data: vec!["1".into(), "2".into(), "3".into()],
                update_time: 0,
            })
        );
        key.ns = 0;
        assert_noop!(
            Gmetadata::remove_index(u1.into(), key.clone(), "1".into(), req_id),
            Error::<Test>::NamespaceNotFound
        );
        key.ns = 1;
        assert_ok!(Gmetadata::remove_index(
            u1.into(),
            key.clone(),
            "1".into(),
            req_id
        ));
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
        let mut req_id = H256::default();

        assert_ok!(Gmetadata::add_index(
            u1.into(),
            key.clone(),
            "1".into(),
            req_id
        ));
        assert_ok!(Gmetadata::add_index(
            u1.into(),
            key.clone(),
            "2".into(),
            req_id
        ));
        key.pk = "1".into();
        assert_ok!(Gmetadata::set_value(
            u1.into(),
            key.clone(),
            "network1".into(),
            req_id
        ));
        key.pk = "2".into();
        assert_ok!(Gmetadata::set_value(
            u1.into(),
            key.clone(),
            "network2".into(),
            req_id
        ));
        key.pk = "".into();
        assert_eq!(
            Gmetadata::query_with_index(key.clone(), key.clone(), "".into(), 10),
            GmetadataQueryResult {
                list: vec!["network1".into(), "network2".into()],
                cursor: "".into(),
            }
        );
        assert_eq!(
            Gmetadata::query_with_index(key.clone(), key.clone(), "".into(), 1),
            GmetadataQueryResult {
                list: vec!["network1".into()],
                cursor: "1".into(),
            }
        );
        assert_eq!(
            Gmetadata::query_with_index(key.clone(), key.clone(), "1".into(), 1),
            GmetadataQueryResult {
                list: vec!["network2".into()],
                cursor: "".into(),
            }
        );
    })
}
