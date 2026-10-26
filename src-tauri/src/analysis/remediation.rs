use crate::domain::{Category, EventType, SystemEvent};

pub fn advice(event: &SystemEvent) -> &'static str {
    match (event.event.category, event.event.event_type) {
        (Category::Network, EventType::ConfigurationChanged) => {
            "Check your proxy, DNS, VPN, and adapter settings in Windows Settings."
        }
        (Category::Driver, EventType::Updated | EventType::VersionChanged) => {
            "Compare the driver version with the version from when this worked."
        }
        (Category::Service, EventType::Stopped | EventType::Failed) => {
            "Check the service state and its recent Windows Event Log entries."
        }
        (Category::Configuration, _) => {
            "Compare this configuration against a checkpoint from when this worked."
        }
        _ => "Compare against a checkpoint from when this worked.",
    }
}
