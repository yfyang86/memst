#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Content, ContentPart, Message, Role};

    #[test]
    fn test_message_creation() {
        let msg = Message::new(Role::User, "Hello, world!");
        assert_eq!(msg.role, Role::User);
        match &msg.content {
            Content::Text(s) => assert_eq!(s, "Hello, world!"),
            _ => panic!("Expected Text content"),
        }
        assert!(msg.id != Uuid::nil());
        assert!(msg.token_count.is_none());
    }

    #[test]
    fn test_message_with_token_count() {
        let msg = Message::new(Role::Assistant, "Response")
            .with_token_count(42);
        assert_eq!(msg.token_count, Some(42));
    }

    #[test]
    fn test_message_with_metadata() {
        let msg = Message::new(Role::User, "Test")
            .with_metadata("model", "gpt-4")
            .with_metadata("temperature", 0.7);
        assert_eq!(msg.metadata.len(), 2);
        assert_eq!(msg.metadata.get("model").unwrap(), "gpt-4");
    }

    #[test]
    fn test_message_serialization_roundtrip() {
        let msg = Message {
            id: Uuid::new_v4(),
            role: Role::User,
            content: Content::Text("Hello".to_string()),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
            token_count: Some(10),
        };

        let options = bincode::DefaultOptions::new()
            .with_fixint_encoding()
            .allow_trailing_bytes();
        let serialized = options.serialize(&msg).unwrap();
        let deserialized: Message = options.deserialize(&serialized).unwrap();

        assert_eq!(msg.id, deserialized.id);
        assert_eq!(msg.role, deserialized.role);
        assert_eq!(msg.content, deserialized.content);
    }

    #[test]
    fn test_multipart_content() {
        let content = Content::MultiPart(vec![
            ContentPart::Text("Hello".to_string()),
            ContentPart::Image {
                data: vec![0x89, 0x50, 0x4E],
                mime_type: "image/png".to_string(),
            },
        ]);

        let options = bincode::DefaultOptions::new()
            .with_fixint_encoding()
            .allow_trailing_bytes();
        let serialized = options.serialize(&content).unwrap();
        let deserialized: Content = options.deserialize(&serialized).unwrap();

        match deserialized {
            Content::MultiPart(parts) => assert_eq!(parts.len(), 2),
            _ => panic!("Expected MultiPart"),
        }
    }

    #[test]
    fn test_session_metadata() {
        let metadata = SessionMetadata::new("Test Session", "gpt-4")
            .with_tag("test")
            .with_tag("debug");

        assert_eq!(metadata.name, "Test Session");
        assert_eq!(metadata.model, "gpt-4");
        assert_eq!(metadata.tags.len(), 2);
        assert!(metadata.created_at <= Utc::now());
    }

    #[test]
    fn test_manifest() {
        let mut manifest = Manifest::new();
        assert_eq!(manifest.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(manifest.sessions.len(), 0);

        let summary = SessionSummary {
            id: Uuid::new_v4(),
            name: "Test".to_string(),
            model: "gpt-4".to_string(),
            tags: vec![],
            created_at: Utc::now(),
            last_activity: Utc::now(),
            message_count: 5,
        };

        manifest.upsert_session(summary.clone());
        assert_eq!(manifest.sessions.len(), 1);
        assert_eq!(manifest.get_session(&summary.id).unwrap().message_count, 5);

        manifest.remove_session(&summary.id);
        assert_eq!(manifest.sessions.len(), 0);
    }

    #[test]
    fn test_role_display() {
        assert_eq!(Role::System.to_string(), "system");
        assert_eq!(Role::User.to_string(), "user");
        assert_eq!(Role::Assistant.to_string(), "assistant");
        assert_eq!(Role::Tool.to_string(), "tool");
    }

    #[test]
    fn test_content_from_strings() {
        let text: Content = "Hello".into();
        match text {
            Content::Text(s) => assert_eq!(s, "Hello"),
        }
    }
}
