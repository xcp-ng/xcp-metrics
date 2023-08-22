use std::time::SystemTime;

use xcp_metrics_common::rrdd::rrd_updates::RrdXport;

fn main() {
    let rrd = RrdXport {
        start: SystemTime::now(),
        end: SystemTime::now(),
        step_secs: 5,
        legend: vec!["test".into(), "test2".into(), "test3".into()],
        data: vec![
            (SystemTime::now(), vec![1.0, 2.0, 3.0].into()),
            (SystemTime::now(), vec![2.0, 3.0, 4.0].into()),
        ],
    };

    let mut xml = vec![];
    rrd.write_xml(&mut xml).unwrap();

    println!("{}", String::from_utf8(xml).unwrap());
}
