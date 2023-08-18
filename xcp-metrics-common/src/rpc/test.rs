mod methods {
    use dxr::{TryFromValue, TryToValue};
    use serde::{Deserialize, Serialize};

    use crate::{
        rpc::{message::RpcRequest, parse_method_jsonrpc, parse_method_xmlrpc, XcpRpcMethod},
        rpc_method,
    };

    #[derive(Debug, Clone, TryToValue, TryFromValue, Serialize, Deserialize, PartialEq)]
    pub struct AnotherStruct {
        value: i32,
    }

    // Use a quite complex structure.
    #[derive(Debug, Clone, TryToValue, TryFromValue, Serialize, Deserialize, PartialEq)]
    pub struct TestRpcMethod {
        pub str: String,
        pub number: i32,
        pub another_struct: AnotherStruct,
    }

    rpc_method!(TestRpcMethod, "Test.Rpc.Method");

    /// Test if XML-RPC content stays the same after being converted back and forth to RPC.
    #[test]
    pub fn invariant_xmlrpc() {
        let test_method = TestRpcMethod {
            str: "test".to_string(),
            number: 42,
            another_struct: AnotherStruct { value: 123 },
        };

        let mut xml_buffer = vec![];
        test_method.write_xmlrpc(&mut xml_buffer).unwrap();

        let xml = String::from_utf8(xml_buffer).unwrap();
        let request = RpcRequest::XmlRpc(parse_method_xmlrpc(&xml).unwrap());

        assert_eq!(
            request.try_into_method::<TestRpcMethod>().unwrap(),
            test_method
        );
    }

    /// Test if JSON-RPC content stays the same after being converted back and forth to RPC.
    #[test]
    pub fn invariant_json() {
        let test_method = TestRpcMethod {
            str: "test".to_string(),
            number: 42,
            another_struct: AnotherStruct { value: 123 },
        };

        let mut jsonrpc_buffer = vec![];
        test_method.write_jsonrpc(&mut jsonrpc_buffer).unwrap();

        let json = String::from_utf8(jsonrpc_buffer).unwrap();
        let request = RpcRequest::JsonRpc(parse_method_jsonrpc(&json).unwrap());

        assert_eq!(
            request.try_into_method::<TestRpcMethod>().unwrap(),
            test_method
        );
    }
}
