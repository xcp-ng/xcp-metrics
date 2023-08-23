use xcp_metrics_plugin_common::xenstore::{
    mock::MockXs,
    xs::{XBTransaction, XsTrait},
};

use crate::SqueezedInfo;

#[test]
fn no_vm() {
    // No virtual machine : all 0
    let xs = MockXs::default();

    xs.write(XBTransaction::Null, "/local/domain", "").unwrap();

    assert_eq!(
        SqueezedInfo::get(&xs).unwrap(),
        SqueezedInfo {
            reclaimed: 0,
            reclaimed_max: 0
        }
    );
}

#[test]
fn single_vm() {
    let xs = MockXs::default();

    xs.write(
        XBTransaction::Null,
        "/local/domain/0/memory/target",
        "123456",
    )
    .unwrap();
    xs.write(
        XBTransaction::Null,
        "/local/domain/0/memory/dynamic-min",
        "0",
    )
    .unwrap();
    xs.write(
        XBTransaction::Null,
        "/local/domain/0/memory/dynamic-max",
        "654321",
    )
    .unwrap();

    assert_eq!(
        SqueezedInfo::get(&xs).unwrap(),
        SqueezedInfo {
            reclaimed: 530865,
            reclaimed_max: 123456
        }
    );
}

#[test]
fn multiple_vm() {
    let xs = MockXs::default();

    xs.write(
        XBTransaction::Null,
        "/local/domain/0/memory/target",
        "123456",
    )
    .unwrap();
    xs.write(
        XBTransaction::Null,
        "/local/domain/0/memory/dynamic-min",
        "0",
    )
    .unwrap();
    xs.write(
        XBTransaction::Null,
        "/local/domain/0/memory/dynamic-max",
        "654321",
    )
    .unwrap();

    // Consider missing domain 1.

    xs.write(
        XBTransaction::Null,
        "/local/domain/2/memory/target",
        "111111",
    )
    .unwrap();
    xs.write(
        XBTransaction::Null,
        "/local/domain/2/memory/dynamic-min",
        "0",
    )
    .unwrap();
    xs.write(
        XBTransaction::Null,
        "/local/domain/2/memory/dynamic-max",
        "999999",
    )
    .unwrap();

    assert_eq!(
        SqueezedInfo::get(&xs).unwrap(),
        SqueezedInfo {
            reclaimed: 530865 + 888888,
            reclaimed_max: 123456 + 111111
        }
    );
}