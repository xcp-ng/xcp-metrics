mod methods {
    use dxr::{TryFromValue, TryToValue};
    use serde::{Deserialize, Serialize};

    use crate::{
        rpc::{
            message::{RpcKind, RpcRequest},
            write_method_jsonrpc, write_method_xmlrpc,
        },
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

        let mut xml = vec![];
        write_method_xmlrpc(&mut xml, &test_method).unwrap();

        let request = RpcRequest::parse(&xml, RpcKind::XmlRpc).unwrap();

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

        let mut json = vec![];
        write_method_jsonrpc(&mut json, &test_method).unwrap();

        let request = RpcRequest::parse(&json, RpcKind::JsonRpc).unwrap();

        assert_eq!(
            request.try_into_method::<TestRpcMethod>().unwrap(),
            test_method
        );
    }
}
