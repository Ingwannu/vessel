//! The fake provider's framed load acknowledgement, shared by real admission tests.
use super::*;
use tokio::net::TcpStream;
pub(super) struct LoadGate {
    listener: TcpListener,
}
impl LoadGate {
    pub(super) async fn install(f: &Fixture) -> Self {
        Self::install_env(f, "FAKE_ACP_LOAD_GATE").await
    }
    pub(super) async fn install_prompt(f: &Fixture) -> Self {
        Self::install_env(f, "FAKE_ACP_PROMPT_GATE").await
    }
    async fn install_env(f: &Fixture, variable: &str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        f.profile_env(&format!("env = {{ {variable} = \"{address}\" }}\n"));
        Self { listener }
    }
    pub(super) async fn arrive(&self) -> (String, TcpStream) {
        tokio::time::timeout(WAIT, async {
            let (mut stream, _) = self.listener.accept().await.unwrap();
            let length = stream.read_u32().await.unwrap();
            assert!(
                usize::try_from(length).unwrap() <= vessel_testkit::LOAD_GATE_SESSION_MAX_BYTES
            );
            let mut bytes = vec![0; usize::try_from(length).unwrap()];
            assert_eq!(stream.read_exact(&mut bytes).await.unwrap(), bytes.len());
            (String::from_utf8(bytes).unwrap(), stream)
        })
        .await
        .expect("provider reaches the framed load barrier")
    }
}
