//! A field read shares transport admission without sending its patch probe.

use futures_util::{SinkExt as _, StreamExt as _};
use vessel_proto::settings::{GetNodeSettings, NodeSettingKey, ReloadNodeSettings};
use vessel_proto::{Method, ProtocolVersion, Request as _, ServerFrame, Welcome};

use super::*;
use crate::CallError;

const PREFLIGHT_TEST_BUDGET: std::time::Duration = std::time::Duration::from_secs(5);

#[tokio::test]
async fn settings_field_preflight_refuses_old_and_pending_data_without_a_frame() {
    let result = tokio::time::timeout(PREFLIGHT_TEST_BUDGET, async {
        let key = NodeSettingKey::Drivers;
        let needs = key.required_protocol().unwrap();
        let older = ProtocolVersion::new(needs.get().checked_sub(1).unwrap());
        for negotiated in [older, needs] {
            let (client_end, mut settings_peer) = vessel_rpc::memory::pair(NonZeroUsize::MIN);
            let observing = tokio::spawn(async move {
                let _greeting = settings_peer.next().await.unwrap().unwrap();
                let welcome = ServerFrame::Welcome(Welcome {
                    protocol: negotiated,
                    node: node(),
                });
                settings_peer
                    .send(serde_json::to_string(&welcome).unwrap())
                    .await
                    .unwrap();
                settings_peer.next().await
            });
            let client = NodeClient::over(client_end, client_info(), &ClientConfig::default())
                .await
                .unwrap();
            let checked = client.check_call(&key.read_requirement().unwrap());
            if negotiated < needs {
                assert_eq!(
                    checked,
                    Err(CallError::NotInProtocol {
                        method: Method::SetNodeSettings,
                        needs,
                        negotiated,
                    })
                );
            } else {
                assert_eq!(checked, Ok(()));
            }
            assert_eq!(client.check_call(&GetNodeSettings {}.into_call()), Ok(()));
            let reload = client.check_call(&ReloadNodeSettings {}.into_call());
            match vessel_proto::settings::NODE_CONFIG_PROTOCOL {
                None => assert_eq!(
                    reload,
                    Err(CallError::UnassignedProtocol {
                        method: Method::ReloadNodeSettings
                    })
                ),
                Some(required) if negotiated < required => assert_eq!(
                    reload,
                    Err(CallError::NotInProtocol {
                        method: Method::ReloadNodeSettings,
                        needs: required,
                        negotiated,
                    })
                ),
                Some(_) => assert_eq!(reload, Ok(())),
            }
            assert!(NodeSettingKey::Unknown.read_requirement().is_err());
            drop(client);
            assert!(
                observing.await.unwrap().is_none(),
                "preflight sent a request"
            );
        }
    })
    .await;
    assert!(
        result.is_ok(),
        "settings preflight exceeded {PREFLIGHT_TEST_BUDGET:?}"
    );
}
