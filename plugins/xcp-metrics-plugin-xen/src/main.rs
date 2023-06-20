use xcp_metrics_common::xmlrpc::{PluginLocalRegister, XcpRpcMethod};

fn main() {
    let request = PluginLocalRegister {
        info: "Five_Seconds".into(),
        protocol: "V2".into(),
        uid: "xcp-metrics-plugin-xen".into()
    };

    let mut buffer = vec![];
    println!("{:?}", request.write_xmlrpc(&mut buffer));

    println!("{:}", String::from_utf8_lossy(&buffer));
}
