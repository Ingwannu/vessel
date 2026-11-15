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

struct HeldWrite {
    sent: tokio::sync::mpsc::Sender<String>,
    release: Arc<tokio::sync::Semaphore>,
    replies: tokio::sync::mpsc::Receiver<String>,
}
impl Transport for HeldWrite {
    async fn send(&mut self, text: String) -> Result<(), CdpError> {
        self.sent.send(text).await.unwrap();
        let permit = self.release.acquire().await.unwrap();
        permit.forget();
        Ok(())
    }
    async fn next(&mut self) -> Result<Option<String>, CdpError> {
        Ok(self.replies.recv().await)
    }
}
#[tokio::test]
async fn withdrawn_queued_command_cannot_write_after_its_not_delivered_timeout() {
    const PEER_CAPACITY: usize = 4;
    const QUEUED_BUDGET: Duration = Duration::from_millis(1);
    tokio::time::timeout(FIXTURE_TIMEOUT, async {
        let (sent, mut writes) = mpsc::channel(PEER_CAPACITY);
        let (reply, replies) = mpsc::channel(PEER_CAPACITY);
        let release = Arc::new(tokio::sync::Semaphore::new(0));
        let (client, _events) = Client::start(HeldWrite {sent, release: Arc::clone(&release), replies});
        let first_client = client.clone();
        let first = tokio::spawn(async move {first_client.execute(None,requests::Version{}).await});
        let first_write = writes.recv().await.unwrap();
        let first_wire: serde_json::Value = serde_json::from_str(&first_write).unwrap();
        assert!(matches!(
            client.execute_with_budget(None, requests::Version{}, QUEUED_BUDGET).await,
            Err(CdpError::TimedOut{effect:BrowserEffect::NotDelivered,..})
        ));
        release.add_permits(1);
        let response = serde_json::json!({"id":first_wire["id"],"result":{"protocolVersion":"fixture","product":"not-qualified","revision":"fixture","userAgent":"fixture","jsVersion":"fixture"}});
        reply.send(response.to_string()).await.unwrap();
        first.await.unwrap().unwrap();
        client.shutdown().await.unwrap();
        // Joined driver: there is no remaining worker that could publish a late write.
        assert!(writes.try_recv().is_err());
    }).await.unwrap();
}
