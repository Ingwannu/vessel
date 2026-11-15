//! Settings › Connections: a route shows where it goes and nothing else
//! of its URL.

use crate::connections::{Endpoint, Route};

#[test]
fn keeps_only_the_host_and_port_of_a_url() {
    for (url, shown) in [
        ("ws://127.0.0.1:7077/rpc", "127.0.0.1:7077"),
        (
            "wss://relay.example:443/r/node-route?x=1#y",
            "relay.example:443",
        ),
        ("wss://someone:pass@relay.example/secure", "relay.example"),
        ("10.0.0.7:7077", "10.0.0.7:7077"),
        ("ws://[::1]:7077/rpc", "[::1]:7077"),
    ] {
        assert_eq!(Endpoint::of_url(url).as_str(), shown, "{url}");
    }
}

#[test]
fn names_each_route_by_where_it_goes() {
    let relay = Route::Relay(Endpoint::of_url("wss://relay.example:443/r/abc"));
    assert_eq!(relay.endpoint().as_str(), "relay.example:443");
    let ssh = Route::Ssh(Endpoint::of_ssh("me@gpu-box:2222"));
    assert_eq!(ssh.endpoint().as_str(), "me@gpu-box:2222");
}

#[test]
fn labels_each_route_as_paseo_does() {
    use crate::Lang;
    let routes = [
        Route::Tcp(Endpoint::of_url("ws://127.0.0.1:7077/rpc")),
        Route::Direct(Endpoint::of_url("ws://10.0.0.7:7077")),
        Route::Relay(Endpoint::of_url("wss://relay.example:443/r/abc")),
        Route::Ssh(Endpoint::of_ssh("me@gpu-box")),
    ];
    let label = |lang: Lang| -> Vec<String> {
        routes
            .iter()
            .map(|route| lang.overview().route(route))
            .collect()
    };
    assert_eq!(
        label(Lang::En),
        [
            "TCP (127.0.0.1:7077)",
            "TCP (10.0.0.7:7077)",
            "Relay (relay.example:443)",
            "Remote SSH (me@gpu-box)",
        ]
    );
    assert_eq!(
        label(Lang::Ko),
        [
            "TCP (127.0.0.1:7077)",
            "TCP (10.0.0.7:7077)",
            "릴레이 (relay.example:443)",
            "원격 SSH (me@gpu-box)",
        ]
    );
}
