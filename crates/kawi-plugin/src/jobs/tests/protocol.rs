use serde_json::Value;

use super::*;

#[test]
fn exit_event_payload_is_json() {
    let (manager, receiver) = manager("json", &["/bin/sh"]);
    let job = open(&manager, "json", "/bin/sh", &["-c", "true"]);
    let exited = wait_exit(&receiver, job);
    assert_eq!(
        serde_json::to_value(exited).expect("json"),
        Value::Object(
            [
                ("exit_code".to_string(), Value::from(0)),
                ("job".to_string(), Value::from(job)),
                ("signal".to_string(), Value::Null),
                ("state".to_string(), Value::from("exited")),
            ]
            .into_iter()
            .collect()
        )
    );
}
