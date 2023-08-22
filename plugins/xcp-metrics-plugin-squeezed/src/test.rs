use xcp_metrics_plugin_common::xenstore::{mock::MockXs, xs::{XsTrait, XBTransaction}};

use crate::SqueezedInfo;

#[test]
fn test_no_vm() {
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
fn test_single_vm() {
    let xs = MockXs::default();

    xs.write(XBTransaction::Null, "/local/domain", "").unwrap();
    xs.write(XBTransaction::Null, "/local/domain/0/memory/", "").unwrap();
    xs.write(XBTransaction::Null, "/local/domain/0/memory/target", "123456").unwrap();
    xs.write(XBTransaction::Null, "/local/domain/0/memory/dynamic-min", "0").unwrap();
    xs.write(XBTransaction::Null, "/local/domain/0/memory/dynamic-max", "654321").unwrap();

    assert_eq!(
        SqueezedInfo::get(&xs).unwrap(),
        SqueezedInfo {
            reclaimed: 123456,
            reclaimed_max: 555555
        }
    );
}