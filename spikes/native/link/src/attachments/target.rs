//! A book label is an address; only the welcome `NodeId` pins the destination.
use super::TargetProblem;
use vessel_client::NodeClient;
use vessel_proto::NodeId;

fn check(client: &NodeClient, expected: NodeId) -> Result<(), TargetProblem> {
    let actual = client.node().id;
    if actual == expected {
        Ok(())
    } else {
        Err(TargetProblem::Changed { expected, actual })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn a_reused_alias_never_sends_an_attachment_request_to_the_replacement_peer() {
        let (client, peer) = crate::attachments::read::tests::peer::connect(Vec::new()).await;
        let actual = client.node().id;
        assert_eq!(check(&client, actual), Ok(()));
        let expected = NodeId::random();
        assert_eq!(
            check(&client, expected),
            Err(TargetProblem::Changed { expected, actual })
        );
        drop(client);
        assert!(peer.await.unwrap().is_empty());
    }
}
