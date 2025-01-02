mod methods {
    use dxr::{TryFromValue, TryToValue};
    use serde::{Deserialize, Serialize};

    use crate::{
        rpc::message::{parse_method_jsonrpc, parse_method_xmlrpc, request::RpcRequest, RpcKind},
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

        let xml = RpcRequest::new(&test_method, RpcKind::XmlRpc)
            .unwrap()
            .to_body()
            .unwrap();

        println!("{xml}");

        assert_eq!(
            parse_method_xmlrpc::<TestRpcMethod>(xml.as_bytes()).unwrap(),
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

        let json = RpcRequest::new(&test_method, RpcKind::JsonRpc)
            .unwrap()
            .to_body()
            .unwrap();

        assert_eq!(
            parse_method_jsonrpc::<TestRpcMethod>(json.as_bytes()).unwrap(),
            test_method
        );
    }
}
