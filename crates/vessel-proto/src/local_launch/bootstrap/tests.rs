use super::*;

fn challenge() -> BootstrapChallenge {
    BootstrapChallenge::from_bytes([0xa5; CHALLENGE_BYTES])
}

#[test]
fn private_records_have_exact_bounds_and_round_trip_without_debug_payload() {
    let challenge = challenge();
    let encoded = challenge.encode();
    assert_eq!(encoded.len(), CHALLENGE_RECORD_BYTES);
    let decoded = BootstrapChallenge::decode(&encoded).unwrap();
    assert_eq!(decoded.as_bytes(), challenge.as_bytes());
    let ready = BootstrapReady::new(NodeId::random(), challenge);
    let record = ready.encode();
    assert_eq!(record.len(), MAX_BOOTSTRAP_READY_BYTES);
    let decoded = BootstrapReady::decode(&record).unwrap();
    assert_eq!(decoded.node(), ready.node());
    assert_eq!(decoded.challenge().as_bytes(), ready.challenge().as_bytes());
    assert_eq!(format!("{challenge:?}"), "BootstrapChallenge(<redacted>)");
    assert!(!format!("{ready:?}").contains(&URL_SAFE_NO_PAD.encode(challenge.0)));
}

#[test]
fn extra_lines_versions_padding_and_noncanonical_ids_are_refused() {
    let encoded = challenge().encode();
    for bad in [
        encoded.trim_end().to_owned(),
        encoded.replace("/1 ", "/2 "),
        format!("log\n{encoded}"),
        format!("{encoded}{encoded}"),
        encoded.replace('\n', "=\n"),
        encoded.replace('p', "+"),
    ] {
        assert!(
            BootstrapChallenge::decode(&bad).is_err(),
            "accepted malformed challenge"
        );
    }
    let node: NodeId = "deadbeef-abcd-4abc-8abc-123456abcdef".parse().unwrap();
    let ready = BootstrapReady::new(node, challenge()).encode();
    for bad in [
        ready.trim_end().to_owned(),
        ready.replace("/2 ", "/1 "),
        ready.replace(&node.to_string(), &node.to_string().to_uppercase()),
        format!("{ready}extra\n"),
        format!("log\n{ready}"),
        ready.replace(' ', "  "),
    ] {
        assert!(
            BootstrapReady::decode(&bad).is_err(),
            "accepted malformed stdout echo"
        );
    }
}

#[test]
fn a_stale_record_preserves_its_own_challenge_for_caller_validation() {
    let node = NodeId::random();
    let old = BootstrapReady::new(node, challenge());
    let fresh = BootstrapChallenge::from_bytes([0x5a; CHALLENGE_BYTES]);
    let decoded = BootstrapReady::decode(&old.encode()).unwrap();
    assert_ne!(decoded.challenge().as_bytes(), fresh.as_bytes());
    // Equality here is a test assertion only. Production verifies in constant
    // time after correlating this record with the retained child stdout.
    assert_eq!(decoded.node(), node);
}
