//! Pure Docker Engine protocol acceptance checks. No daemon is required.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::io::Cursor;

use what_changed_lib::collectors::docker_protocol::{
    parse_containers_response, parse_event_line, parse_event_stream, parse_images_response,
    parse_version_response, DockerAction, DockerApiVersion, DockerObjectKind, DockerProtocolError,
    DockerRequest, DOCKER_PROTOCOL_SCHEMA_VERSION, FALLBACK_API_VERSION, MAX_EVENT_LINE_BYTES,
    MAX_RESPONSE_BYTES, PRIMARY_API_VERSION,
};

const EVENTS: &[u8] = include_bytes!("fixtures/docker/events-stream.ndjson");
const CONTAINERS: &[u8] = include_bytes!("fixtures/docker/containers.json");
const IMAGES: &[u8] = include_bytes!("fixtures/docker/images.json");
const VERSION: &[u8] = include_bytes!("fixtures/docker/version.json");

#[test]
fn read_only_request_targets_are_versioned_and_closed() {
    assert_eq!(PRIMARY_API_VERSION, DockerApiVersion::V1_43);
    assert_eq!(FALLBACK_API_VERSION, DockerApiVersion::V1_41);
    assert_eq!(DockerRequest::Ping.path(PRIMARY_API_VERSION), "/_ping");
    assert_eq!(
        DockerRequest::Version.path(FALLBACK_API_VERSION),
        "/v1.41/version"
    );
    assert_eq!(
        DockerRequest::Containers.path(PRIMARY_API_VERSION),
        "/v1.43/containers/json?all=1"
    );
    assert_eq!(
        DockerRequest::Images.path(PRIMARY_API_VERSION),
        "/v1.43/images/json"
    );
    let events = DockerRequest::Events.path(PRIMARY_API_VERSION);
    assert!(events.starts_with("/v1.43/events?filters="));
    assert!(events.contains("container"));
}

#[test]
fn every_fixture_event_maps_to_a_typed_protocol_event() {
    let events = parse_event_stream(Cursor::new(EVENTS)).unwrap();
    assert_eq!(events.len(), 11);
    assert!(events
        .iter()
        .all(|event| event.schema_version == DOCKER_PROTOCOL_SCHEMA_VERSION));
    let kinds: Vec<_> = events
        .iter()
        .map(|event| (event.object_kind, event.action))
        .collect();
    assert_eq!(
        kinds,
        vec![
            (DockerObjectKind::Container, DockerAction::Created),
            (DockerObjectKind::Container, DockerAction::Started),
            (DockerObjectKind::Container, DockerAction::Stopped),
            (DockerObjectKind::Container, DockerAction::Died),
            (DockerObjectKind::Container, DockerAction::Removed),
            (DockerObjectKind::Image, DockerAction::Pulled),
            (DockerObjectKind::Image, DockerAction::Removed),
            (DockerObjectKind::Volume, DockerAction::Created),
            (DockerObjectKind::Volume, DockerAction::Removed),
            (DockerObjectKind::Network, DockerAction::Created),
            (DockerObjectKind::Network, DockerAction::Removed),
        ]
    );

    let started = &events[1];
    assert_eq!(started.display_name, "wc-db");
    assert_eq!(started.compose_project.as_deref(), Some("what-changed"));
    assert!(started.published_ports.is_empty());
    assert_eq!(started.occurred_at_ms, 1_725_000_001_000);

    let died = &events[3];
    assert_eq!(died.action, DockerAction::Died);
    assert_eq!(died.exit_code, Some(137));
}

#[test]
fn baseline_and_version_fixtures_are_bounded_typed_data() {
    let version = parse_version_response(VERSION).unwrap();
    assert_eq!(version.engine_version, "27.2.0");
    assert_eq!(version.api_version, "1.47");
    assert_eq!(version.minimum_api_version, "1.24");

    let containers = parse_containers_response(CONTAINERS).unwrap();
    assert_eq!(containers.len(), 2);
    assert_eq!(containers[0].names, ["/wc-db"]);
    assert_eq!(
        containers[0].compose_project.as_deref(),
        Some("what-changed")
    );
    assert_eq!(containers[0].published_ports[0].host_port, 5432);
    assert!(!containers[0].exit_code_label_present);

    let images = parse_images_response(IMAGES).unwrap();
    assert_eq!(images.len(), 2);
    assert_eq!(images[1].repository_tags, ["wc-api:dev", "wc-api:latest"]);
}

#[test]
fn unknown_events_are_forward_compatible_but_invalid_schema_is_rejected() {
    let unknown = br#"{"Type":"container","Action":"health_status","Actor":{"ID":"abc","Attributes":{}},"time":1}"#;
    assert_eq!(parse_event_line(unknown).unwrap(), None);

    let bad_exit = br#"{"Type":"container","Action":"die","Actor":{"ID":"abc","Attributes":{"name":"db","exitCode":"secret-value"}},"time":1}"#;
    let parsed = parse_event_line(bad_exit).unwrap().unwrap();
    assert_eq!(parsed.exit_code, None);
}

#[test]
fn malformed_and_oversized_input_fails_without_echoing_payload() {
    let secret = br#"{"Type":"container","Action":"start","password":"never-log-this""#;
    let error = parse_event_line(secret).unwrap_err();
    assert_eq!(error, DockerProtocolError::MalformedJson);
    assert!(!error.to_string().contains("never-log-this"));

    let oversized = vec![b'x'; MAX_EVENT_LINE_BYTES + 1];
    assert_eq!(
        parse_event_line(&oversized).unwrap_err(),
        DockerProtocolError::LineTooLarge
    );

    let response = vec![b' '; MAX_RESPONSE_BYTES + 1];
    assert_eq!(
        parse_version_response(&response).unwrap_err(),
        DockerProtocolError::ResponseTooLarge
    );
}

#[test]
fn oversized_stream_frame_is_rejected_while_incrementally_reading() {
    let oversized = vec![b'x'; MAX_EVENT_LINE_BYTES + 1];
    let reader = std::io::BufReader::with_capacity(31, Cursor::new(oversized));
    assert_eq!(
        parse_event_stream(reader).unwrap_err(),
        DockerProtocolError::LineTooLarge
    );
}

#[test]
fn disconnect_fixture_reports_only_the_truncated_frame_as_malformed() {
    let fixture = include_bytes!("fixtures/docker/events-with-disconnect.ndjson");
    assert_eq!(
        parse_event_stream(Cursor::new(fixture)).unwrap_err(),
        DockerProtocolError::MalformedJson
    );
}

#[test]
fn baseline_ports_keep_supported_protocols_and_skip_unknown_protocols() {
    let bad_protocol = br#"[{"Id":"abc","Names":["/db"],"Image":"db:1","State":"running","Status":"Up","Ports":[{"IP":"127.0.0.1","PrivatePort":5432,"PublicPort":5432,"Type":"sctp"}]}]"#;
    assert!(parse_containers_response(bad_protocol)
        .unwrap()
        .first()
        .unwrap()
        .published_ports
        .is_empty());

    let control_address = b"[{\"Id\":\"abc\",\"Names\":[\"/db\"],\"Image\":\"db:1\",\"State\":\"running\",\"Status\":\"Up\",\"Ports\":[{\"IP\":\"127.0.0.1\\nsecret\",\"PrivatePort\":5432,\"PublicPort\":5432,\"Type\":\"tcp\"}]}]";
    assert_eq!(
        parse_containers_response(control_address).unwrap_err(),
        DockerProtocolError::InvalidSchema("host address")
    );
}

#[test]
fn event_timestamp_is_required_and_cannot_default_to_epoch_zero() {
    let missing = br#"{"Type":"container","Action":"start","Actor":{"ID":"abc","Attributes":{}}}"#;
    assert_eq!(
        parse_event_line(missing).unwrap_err(),
        DockerProtocolError::InvalidSchema("timestamp")
    );
    let zero = br#"{"Type":"container","Action":"start","Actor":{"ID":"abc","Attributes":{}},"time":0,"timeNano":0}"#;
    assert_eq!(
        parse_event_line(zero).unwrap_err(),
        DockerProtocolError::InvalidSchema("timestamp")
    );
}
