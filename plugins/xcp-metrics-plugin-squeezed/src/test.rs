use xcp_metrics_plugin_common::xenstore::mock::MockXs;

use crate::SqueezedInfo;

#[test]
fn test_no_vm() {
    // No virtual machine : all 0
    let xs = MockXs::default();

    assert_eq!(
        SqueezedInfo::get(&xs).unwrap(),
        SqueezedInfo {
            reclaimed: 0,
            reclaimed_max: 0
        }
    );
}
