use conservator::TextEnum;

// Basic usage: variant names become string values
#[derive(Debug, TextEnum)]
enum MessageType {
    Inbound,
    Outbound,
}

// With serde rename_all
#[derive(Debug, TextEnum)]
#[serde(rename_all = "snake_case")]
enum Status {
    ActiveUser,
    InactiveUser,
    PendingApproval,
}

// With individual rename
#[derive(Debug, TextEnum)]
enum Priority {
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "high")]
    High,
}

// Mixed: rename_all with individual override
#[derive(Debug, TextEnum)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum EventType {
    UserCreated,
    UserDeleted,
    #[serde(rename = "CUSTOM_EVENT")]
    CustomEvent,
}

fn main() {
    // These should compile, verifying SqlType is implemented
    use conservator::IntoValue;

    let _ = MessageType::Inbound.into_value();
    let _ = Status::ActiveUser.into_value();
    let _ = Priority::Low.into_value();
    let _ = EventType::UserCreated.into_value();
}
