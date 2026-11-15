//! A private scripted byte peer tests correlation/closure, never a browser engine.
use super::*;
use crate::cdp::{requests, transport};
use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader};
const FIXTURE_TIMEOUT: Duration = Duration::from_secs(20);
#[tokio::test]
async fn typed_driver_uses_the_owned_nul_pipe_without_discovering_a_socket() {
    tokio::time::timeout(FIXTURE_TIMEOUT,async {
        let (node,peer)=tokio::io::duplex(super::super::transport::PIPE_TEST_CAPACITY);
        let (read,write)=tokio::io::split(node);
        let (client,_events)=Client::start(transport::Pipe::new(read,write));
        let peer=tokio::spawn(async move {
            let mut peer=BufReader::new(peer);
            let mut request=Vec::new();let _read = peer.read_until(0,&mut request).await.unwrap();
            assert_eq!(request.pop(),Some(0));
            let request:serde_json::Value=serde_json::from_slice(&request).unwrap();
            assert_eq!(request["method"],"Browser.getVersion");
            let response=serde_json::json!({"id":request["id"],"result":{"protocolVersion":"fixture","product":"not-qualified","revision":"fixture","userAgent":"fixture","jsVersion":"fixture"}});
            peer.get_mut().write_all(response.to_string().as_bytes()).await.unwrap();
            peer.get_mut().write_all(&[0]).await.unwrap();
            let mut closed=[0;1];assert_eq!(peer.read(&mut closed).await.unwrap(),0);
        });
        let result=client.execute(None,requests::Version{}).await.unwrap();
        assert_eq!(result.product.as_str(),"not-qualified");
        client.shutdown().await.unwrap();peer.await.unwrap();
    }).await.unwrap();
}
