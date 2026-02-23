#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Content, Message, Role, SessionMetadata};
    use std::path::Path;
    use tempfile::TempDir;

    fn create_test_store(path: &Path) -> SessionStore {
        SessionStore::init(path).unwrap()
    }

    #[tokio::test]
    async fn test_directory_structure_creation() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());

        assert!(temp_dir.path().join("manifest.json").exists());
        assert!(temp_dir.path().join("schema_version").exists());
        assert!(temp_dir.path().join("store.lock").exists());
        assert!(temp_dir.path().join("sessions").is_dir());
        assert!(temp_dir.path().join("attachments").is_dir());
    }

    #[tokio::test]
    async fn test_schema_version_format() {
        let temp_dir = TempDir::new().unwrap();
        create_test_store(temp_dir.path());

        let version = fs::read_to_string(temp_dir.path().join("schema_version"))
            .await
            .unwrap();
        assert_eq!(version.trim(), "1.0.0");
    }

    #[tokio::test]
    async fn test_create_and_get_session() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());

        let id = store
            .create_session(SessionMetadata::new("Test Session", "gpt-4"))
            .unwrap();

        let sessions = store.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, id);

        let metadata = store.get_session(id).unwrap().unwrap();
        assert_eq!(metadata.name, "Test Session");
        assert_eq!(metadata.model, "gpt-4");
    }

    #[tokio::test]
    async fn test_create_session_with_tags() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());

        let id = store
            .create_session(
                SessionMetadata::new("Test Session", "gpt-4")
                    .with_tag("test")
                    .with_tag("debug"),
            )
            .unwrap();

        let metadata = store.get_session(id).unwrap().unwrap();
        assert_eq!(metadata.tags.len(), 2);
        assert!(metadata.tags.contains(&"test".to_string()));
    }

    #[tokio::test]
    async fn test_delete_session() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());

        let id = store
            .create_session(SessionMetadata::default())
            .unwrap();

        store.delete_session(id).unwrap();

        assert!(store.get_session(id).unwrap().is_none());
        assert_eq!(store.list_sessions().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_append_and_retrieve_messages() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store
            .create_session(SessionMetadata::default())
            .unwrap();

        store
            .append_message(
                session_id,
                Message::new(Role::User, "Hello").with_token_count(5),
            )
            .unwrap();

        store
            .append_message(session_id, Message::new(Role::Assistant, "Hi there!"))
            .unwrap();

        let messages = store.get_messages(session_id).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, Role::User);
        assert_eq!(messages[1].role, Role::Assistant);
    }

    #[tokio::test]
    async fn test_message_range_retrieval() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store
            .create_session(SessionMetadata::default())
            .unwrap();

        for i in 0..10 {
            store
                .append_message(
                    session_id,
                    Message::new(Role::User, format!("Message {}", i)),
                )
                .unwrap();
        }

        let messages = store.get_messages_range(session_id, 5, 8).unwrap();
        assert_eq!(messages.len(), 3);

        // Verify content
        match &messages[0].content {
            Content::Text(s) => assert!(s.contains("5")),
            _ => panic!(),
        }
    }

    #[tokio::test]
    async fn test_index_file_format() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store
            .create_session(SessionMetadata::default())
            .unwrap();

        store
            .append_message(session_id, Message::new(Role::User, "Test"))
            .unwrap();

        let idx_path = store.session_path(session_id).join("messages.idx");
        let content = fs::read_to_string(&idx_path).await.unwrap();

        // Format: message_id byte_offset byte_length timestamp role
        assert!(content.contains("user"));
        // Should contain timestamp
        assert!(content.contains("T"));
    }

    #[tokio::test]
    async fn test_session_metadata_updated() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store
            .create_session(SessionMetadata::default())
            .unwrap();

        store
            .append_message(
                session_id,
                Message::new(Role::User, "Test").with_token_count(10),
            )
            .unwrap();

        let metadata = store.get_session(session_id).unwrap().unwrap();
        assert_eq!(metadata.message_count, 1);
        assert_eq!(metadata.token_count, 10);
    }

    #[tokio::test]
    async fn test_message_order_preserved() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store
            .create_session(SessionMetadata::default())
            .unwrap();

        store
            .append_message(session_id, Message::new(Role::User, "First"))
            .unwrap();
        store
            .append_message(session_id, Message::new(Role::Assistant, "Second"))
            .unwrap();
        store
            .append_message(session_id, Message::new(Role::User, "Third"))
            .unwrap();

        let messages = store.get_messages(session_id).unwrap();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].role, Role::User);
        assert_eq!(messages[1].role, Role::Assistant);
        assert_eq!(messages[2].role, Role::User);
    }

    #[tokio::test]
    async fn test_empty_messages() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store
            .create_session(SessionMetadata::default())
            .unwrap();

        let messages = store.get_messages(session_id).unwrap();
        assert!(messages.is_empty());
    }

    #[tokio::test]
    async fn test_multiple_sessions() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());

        let id1 = store
            .create_session(SessionMetadata::new("Session 1", "gpt-4"))
            .unwrap();
        let id2 = store
            .create_session(SessionMetadata::new("Session 2", "claude-3"))
            .unwrap();

        store
            .append_message(id1, Message::new(Role::User, "From session 1"))
            .unwrap();
        store
            .append_message(id2, Message::new(Role::User, "From session 2"))
            .unwrap();

        let sessions = store.list_sessions().unwrap();
        assert_eq!(sessions.len(), 2);

        let messages1 = store.get_messages(id1).unwrap();
        let messages2 = store.get_messages(id2).unwrap();

        assert_eq!(messages1.len(), 1);
        assert_eq!(messages2.len(), 1);
    }
}
