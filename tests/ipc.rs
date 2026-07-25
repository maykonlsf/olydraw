use std::time::Duration;

use olydraw::ipc::{IpcCommand, bind, send};

fn unique_name(tag: &str) -> String {
    format!("olydraw-test-{}-{}", std::process::id(), tag)
}

#[test]
fn toggle_sent_by_client_reaches_server() {
    let name = unique_name("toggle");
    let (_server, rx) = bind(&name).expect("bind");
    send(&name, IpcCommand::Toggle).expect("send");
    let got = rx
        .recv_timeout(Duration::from_secs(2))
        .expect("command delivered");
    assert_eq!(got, IpcCommand::Toggle);
}

#[test]
fn passthrough_toggle_sent_by_client_reaches_server() {
    let name = unique_name("passthrough");
    let (_server, rx) = bind(&name).expect("bind");
    send(&name, IpcCommand::PassthroughToggle).expect("send");
    let got = rx
        .recv_timeout(Duration::from_secs(2))
        .expect("command delivered");
    assert_eq!(got, IpcCommand::PassthroughToggle);
}

#[test]
fn multiple_toggles_arrive_in_order() {
    let name = unique_name("multi");
    let (_server, rx) = bind(&name).expect("bind");
    send(&name, IpcCommand::Toggle).expect("send 1");
    send(&name, IpcCommand::Toggle).expect("send 2");
    rx.recv_timeout(Duration::from_secs(2)).expect("first");
    rx.recv_timeout(Duration::from_secs(2)).expect("second");
}

#[test]
fn second_bind_on_live_socket_fails() {
    let name = unique_name("double");
    let (_server, _rx) = bind(&name).expect("first bind");
    assert!(
        bind(&name).is_err(),
        "second live instance must be rejected"
    );
}

#[test]
fn send_without_a_server_fails() {
    let name = unique_name("orphan");
    assert!(send(&name, IpcCommand::Toggle).is_err());
}

#[test]
fn socket_is_reusable_after_server_shutdown() {
    let name = unique_name("reuse");
    {
        let (_server, _rx) = bind(&name).expect("first bind");
    }
    let (_server2, rx2) = bind(&name).expect("rebind after drop (stale socket cleanup)");
    send(&name, IpcCommand::Toggle).expect("send to new server");
    rx2.recv_timeout(Duration::from_secs(2)).expect("delivered");
}
