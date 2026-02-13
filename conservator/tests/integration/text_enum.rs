use conservator::{Creatable, Domain, Executor, PooledConnection, TextEnum};
use deadpool_postgres::{Config, PoolConfig};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};
use testcontainers::{Container, clients::Cli};
use testcontainers_modules::postgres::Postgres;

// ========== TextEnum definitions ==========

#[derive(Debug, Clone, PartialEq, TextEnum)]
pub enum MessageType {
    Inbound,
    Outbound,
}

#[derive(Debug, Clone, PartialEq, TextEnum)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Active,
    Inactive,
    PendingReview,
}

#[derive(Debug, Clone, PartialEq, TextEnum)]
pub enum Priority {
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "high")]
    High,
}

// ========== Domain entities ==========

#[derive(Debug, Domain)]
#[domain(table = "messages")]
pub struct Message {
    #[domain(primary_key)]
    pub id: i32,
    pub content: String,
    pub message_type: MessageType,
    pub status: Status,
    pub priority: Priority,
}

#[derive(Debug, Creatable)]
pub struct CreateMessage {
    pub content: String,
    pub message_type: MessageType,
    pub status: Status,
    pub priority: Priority,
}

// ========== Test infrastructure ==========

static DOCKER: OnceLock<Cli> = OnceLock::new();
static CONTAINER: OnceLock<Container<'static, Postgres>> = OnceLock::new();
static DB_COUNTER: AtomicU32 = AtomicU32::new(1000); // Start from 1000 to avoid collision

fn get_container() -> &'static Container<'static, Postgres> {
    let docker = DOCKER.get_or_init(Cli::default);
    CONTAINER.get_or_init(|| docker.run(Postgres::default()))
}

async fn setup_test_db() -> PooledConnection {
    let container = get_container();
    let port = container.get_host_port_ipv4(5432);

    let db_id = DB_COUNTER.fetch_add(1, AtomicOrdering::SeqCst);
    let db_name = format!("test_enum_db_{}", db_id);

    let mut admin_config = Config::new();
    admin_config.host = Some("localhost".to_string());
    admin_config.port = Some(port);
    admin_config.user = Some("postgres".to_string());
    admin_config.password = Some("postgres".to_string());
    admin_config.dbname = Some("postgres".to_string());
    admin_config.pool = Some(PoolConfig {
        max_size: 2,
        ..Default::default()
    });
    let admin_pool = PooledConnection::from_config(admin_config).unwrap();
    let admin_client = admin_pool.get().await.unwrap();

    admin_client
        .execute(&format!("CREATE DATABASE {}", db_name), &[])
        .await
        .unwrap();

    drop(admin_client);

    let mut test_config = Config::new();
    test_config.host = Some("localhost".to_string());
    test_config.port = Some(port);
    test_config.user = Some("postgres".to_string());
    test_config.password = Some("postgres".to_string());
    test_config.dbname = Some(db_name.clone());
    test_config.pool = Some(PoolConfig {
        max_size: 2,
        ..Default::default()
    });
    let pool = PooledConnection::from_config(test_config).unwrap();
    let client = pool.get().await.unwrap();

    client
        .execute(
            r#"
            CREATE TABLE messages (
                id SERIAL PRIMARY KEY,
                content TEXT NOT NULL,
                message_type VARCHAR(50) NOT NULL,
                status VARCHAR(50) NOT NULL,
                priority VARCHAR(50) NOT NULL
            )
            "#,
            &[],
        )
        .await
        .unwrap();

    drop(client);
    pool
}

// ========== Tests ==========

#[tokio::test]
async fn test_text_enum_insert_and_query() {
    let pool = setup_test_db().await;
    let conn = pool.get().await.unwrap();

    let create = CreateMessage {
        content: "Hello".to_string(),
        message_type: MessageType::Inbound,
        status: Status::Active,
        priority: Priority::High,
    };

    let msg: Message = Message::insert(create).returning_entity(&conn).await.unwrap();

    assert_eq!(msg.content, "Hello");
    assert_eq!(msg.message_type, MessageType::Inbound);
    assert_eq!(msg.status, Status::Active);
    assert_eq!(msg.priority, Priority::High);
}

#[tokio::test]
async fn test_text_enum_values_stored_correctly() {
    let pool = setup_test_db().await;
    let conn = pool.get().await.unwrap();

    // Insert with different enum values
    let create = CreateMessage {
        content: "Test".to_string(),
        message_type: MessageType::Outbound,
        status: Status::PendingReview,
        priority: Priority::Low,
    };

    Message::insert(create).returning_pk(&conn).await.unwrap();

    // Query raw values to verify string representation
    let row = conn
        .query_one(
            "SELECT message_type, status, priority FROM messages WHERE content = 'Test'",
            &[],
        )
        .await
        .unwrap();

    let message_type: String = row.get("message_type");
    let status: String = row.get("status");
    let priority: String = row.get("priority");

    // Verify the actual string values stored
    assert_eq!(message_type, "Outbound"); // No rename, uses variant name
    assert_eq!(status, "pending_review"); // rename_all = "snake_case"
    assert_eq!(priority, "low"); // Individual rename
}

#[tokio::test]
async fn test_text_enum_filter() {
    let pool = setup_test_db().await;
    let conn = pool.get().await.unwrap();

    // Insert multiple messages
    for (content, msg_type) in [
        ("msg1", MessageType::Inbound),
        ("msg2", MessageType::Outbound),
        ("msg3", MessageType::Inbound),
    ] {
        let create = CreateMessage {
            content: content.to_string(),
            message_type: msg_type,
            status: Status::Active,
            priority: Priority::Medium,
        };
        Message::insert(create).returning_pk(&conn).await.unwrap();
    }

    // Filter by enum field
    let inbound_messages: Vec<Message> = Message::select()
        .filter(Message::COLUMNS.message_type.eq(MessageType::Inbound))
        .all(&conn)
        .await
        .unwrap();

    assert_eq!(inbound_messages.len(), 2);
    assert!(inbound_messages.iter().all(|m| m.message_type == MessageType::Inbound));
}

#[tokio::test]
async fn test_text_enum_update() {
    let pool = setup_test_db().await;
    let conn = pool.get().await.unwrap();

    let create = CreateMessage {
        content: "Update test".to_string(),
        message_type: MessageType::Inbound,
        status: Status::Active,
        priority: Priority::Low,
    };

    let msg: Message = Message::insert(create).returning_entity(&conn).await.unwrap();

    // Update status
    Message::update()
        .set(Message::COLUMNS.status, Status::Inactive)
        .filter(Message::COLUMNS.id.eq(msg.id))
        .execute(&conn)
        .await
        .unwrap();

    // Verify update
    let updated: Message = Message::fetch_one_by_pk(&msg.id, &conn).await.unwrap();
    assert_eq!(updated.status, Status::Inactive);

    // Verify raw value
    let row = conn
        .query_one(
            "SELECT status FROM messages WHERE id = $1",
            &[&msg.id],
        )
        .await
        .unwrap();
    let status: String = row.get("status");
    assert_eq!(status, "inactive"); // snake_case from rename_all
}

#[tokio::test]
async fn test_text_enum_in_list() {
    let pool = setup_test_db().await;
    let conn = pool.get().await.unwrap();

    // Insert messages with different priorities
    for (content, priority) in [
        ("p1", Priority::Low),
        ("p2", Priority::Medium),
        ("p3", Priority::High),
        ("p4", Priority::Low),
    ] {
        let create = CreateMessage {
            content: content.to_string(),
            message_type: MessageType::Inbound,
            status: Status::Active,
            priority,
        };
        Message::insert(create).returning_pk(&conn).await.unwrap();
    }

    // Query with IN list
    let high_priority: Vec<Message> = Message::select()
        .filter(Message::COLUMNS.priority.in_list(vec![Priority::High, Priority::Medium]))
        .all(&conn)
        .await
        .unwrap();

    assert_eq!(high_priority.len(), 2);
}
