#![allow(clippy::unwrap_used)]
use super::*;
use serde_json::json;
fn transcript() -> Transcript {
    let run:OwnerManualRunDto=serde_json::from_str(r#"{
        "id":"run","app_id":"app","change_id":"change","version":1,"method":"automated","lifecycle":"running","retry_of":null,"started_at":1,"ended_at":null,"execution":null,"manifest_digest":"digest",
        "manifest":{"schema_version":1,"fixture_schema_digest":"fixtures","semantics_version":"owner-browser/v1","rules":[],
            "change":{"id":"change","app_id":"app","version":1,"intent":"Observe test","example":"","scope":"Named check","preserved_rules":[],"candidate_id":"candidate","state":"ready_for_review","updated_at":1},
            "candidate":{"id":"candidate","app_id":"app","environment_id":"environment","environment_revision":1,"identity":"fixture-v1","source":"owner_label","trust":"self_reported","observed_at":1},
            "environment":{"id":"environment","app_id":"app","revision":1,"label":"Test","preview_url":"http://127.0.0.1:5185/","origins":[{"scheme":"http","host":"127.0.0.1","port":5185}],"kind":"local","revoked":false,"created_at":1},
            "scenarios":[{"id":"scenario","app_id":"app","capability_id":"capability","revision":1,"title":"Observe form","expected_behavior":"Visible form","confirmed_by":"Owner","confirmed_at":1,"method":"automated","rules":[],
                "recipe":{"schema_version":1,"role_refs":["staff"],"fixture_refs":[],"adapters":[],"steps":[{"id":"step","action":{"kind":"navigate","path":"/"},"assertions":[{"id":"assertion","expected_behavior":"Form visible","assertion":{"kind":"visible","locator":{"kind":"test_id","value":"form"}}}]}]}}]
        }
    }"#).unwrap();
    Transcript::new(run, "nonce".into(), "scope".into(), "run".into())
}
fn accept(
    transcript: &mut Transcript,
    kind: &str,
    payload: Value,
) -> AppResult<(PreviewRunnerEventDto, Action)> {
    let mut frame = payload;
    for (key,value) in json!({"protocol":1,"run_id":"run","nonce":"nonce","manifest_digest":"digest","sequence":transcript.sequence+1,"type":kind}).as_object().unwrap(){frame[key]=value.clone();}
    transcript.accept(&frame)
}
fn prepare(transcript: &mut Transcript) {
    accept(transcript, "ready", json!({"roles":["staff"]})).unwrap();
    accept(transcript, "needs_login", json!({"role":"staff"})).unwrap();
}
fn action() -> Value {
    json!({"scenario_id":"scenario","scenario_revision":1,"step_id":"step"})
}

#[test]
fn forged_completion_or_observation_without_action_never_establishes_pass() {
    let mut t = transcript();
    prepare(&mut t);
    assert!(accept(
        &mut t,
        "complete",
        json!({"identity":"environment_reported","revision":"fixture-v1"})
    )
    .is_err());
    let mut t = transcript();
    prepare(&mut t);
    accept(&mut t, "before_step", action()).unwrap();
    let mut observation = action();
    observation["assertion_id"] = json!("assertion");
    observation["status"] = json!("passed");
    assert!(accept(&mut t, "assertion", observation).is_err());
}
#[test]
fn exact_assertions_and_terminal_identity_are_required_once() {
    let mut t = transcript();
    prepare(&mut t);
    accept(&mut t, "before_step", action()).unwrap();
    accept(&mut t, "step", action()).unwrap();
    let mut observation = action();
    observation["assertion_id"] = json!("unknown");
    observation["status"] = json!("passed");
    assert!(accept(&mut t, "assertion", observation.clone()).is_err());
    observation["assertion_id"] = json!("assertion");
    accept(&mut t, "assertion", observation.clone()).unwrap();
    assert!(accept(&mut t, "assertion", observation).is_err());
    assert!(accept(
        &mut t,
        "complete",
        json!({"identity":"environment_reported","revision":"stale"})
    )
    .is_err());
    let (_, completion) = accept(
        &mut t,
        "complete",
        json!({"identity":"environment_reported","revision":"fixture-v1"}),
    )
    .unwrap();
    assert!(matches!(completion, Action::Complete(_)));
    assert!(accept(
        &mut t,
        "complete",
        json!({"identity":"environment_reported","revision":"fixture-v1"})
    )
    .is_err());
}
#[test]
fn wrong_run_nonce_digest_sequence_or_unrequested_artifact_is_rejected() {
    for key in ["run_id", "nonce", "manifest_digest"] {
        let mut t = transcript();
        let mut frame = json!({"protocol":1,"run_id":"run","nonce":"nonce","manifest_digest":"digest","sequence":1,"type":"ready","roles":["staff"]});
        frame[key] = json!("forged");
        assert!(t.accept(&frame).is_err());
    }
    let mut t = transcript();
    assert!(accept(
        &mut t,
        "capture",
        json!({"capture_id":"unrequested","role":"staff","data_url":"raw"})
    )
    .is_err());
}
