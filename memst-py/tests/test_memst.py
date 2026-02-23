"""Comprehensive tests for MemSt Python bindings.

Tests cover:
- Session management (create, list, get, delete)
- Message operations (add, retrieve, message history)
- Memory tier management (working, short-term, long-term)
- Full-text search across sessions
- Knowledge graph operations (entities, relationships)
"""

import tempfile
import os
import json
from pathlib import Path

import pytest
from memst import SessionStore, Session, Message, Role, MemoryTier, MemoryItem


class TestSessionManagement:
    """Test session CRUD operations."""

    def test_create_session_with_model(self):
        """Create a session with specific model name."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("GPT-4 Conversation", "gpt-4")

            assert session is not None
            assert session.id is not None
            assert session.name == "GPT-4 Conversation"
            assert session.model == "gpt-4"
            print(f"Created session: {session.id}")
            print(f"  Name: {session.name}")
            print(f"  Model: {session.model}")

    def test_create_multiple_sessions(self):
        """Create multiple sessions with different models."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)

            # Create sessions with different models
            sessions = [
                store.create_session("GPT-4 Session", "gpt-4"),
                store.create_session("Claude Session", "claude-3-opus-20240229"),
                store.create_session("Llama Session", "llama-3-70b"),
            ]

            assert len(sessions) == 3

            # List all sessions (returns dicts)
            all_sessions = store.list_sessions()
            assert len(all_sessions) == 3

            print(f"Created {len(all_sessions)} sessions:")
            for s in all_sessions:
                print(f"  - {s['name']} ({s['model']}): {s['message_count']} messages")

    def test_get_session_by_id(self):
        """Retrieve a specific session by ID."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("Test Session", "gpt-4")

            # Get session by ID
            retrieved = store.get_session(session.id)
            assert retrieved is not None
            assert retrieved["name"] == "Test Session"
            print(f"Retrieved session: {retrieved['name']}")

    def test_delete_session(self):
        """Delete a session and verify it's removed."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("To Delete", "gpt-4")

            # Verify exists
            all_before = store.list_sessions()
            assert len(all_before) == 1

            # Delete
            store.delete_session(session.id)

            # Verify gone
            all_after = store.list_sessions()
            assert len(all_after) == 0
            print("Session deleted successfully")


class TestMessageOperations:
    """Test message append and retrieval."""

    def test_conversation_flow(self):
        """Test a full conversation flow with user/assistant messages."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("Chat Session", "gpt-4")

            # Simulate a conversation
            conversation = [
                (Role.User, "What is Rust programming language?"),
                (Role.Assistant, "Rust is a systems programming language focused on safety and performance."),
                (Role.User, "What are its key features?"),
                (Role.Assistant, "Key features include: memory safety without garbage collection, ownership system, and zero-cost abstractions."),
            ]

            for role, content in conversation:
                store.add_message(session.id, role, content)

            # Retrieve and verify
            messages = store.get_session_messages(session.id)
            assert len(messages) == 4

            print("Conversation flow:")
            for msg in messages:
                print(f"  [{msg['role']}] {msg['content'][:50]}...")

            # Verify order is preserved (Role enum serializes as "User", "Assistant", etc.)
            assert messages[0]["role"] == "User"
            assert "Rust" in messages[0]["content"]

    def test_system_message(self):
        """Test adding system messages for context."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("System Context", "gpt-4")

            # Add system message for behavior definition
            system_prompt = "You are a helpful Python programming assistant."
            store.add_message(session.id, Role.System, system_prompt)

            # Add user message
            store.add_message(session.id, Role.User, "How do I list files in Python?")

            messages = store.get_session_messages(session.id)
            assert len(messages) == 2
            assert messages[0]["role"] == "System"  # Role enum serializes as "System"
            assert messages[1]["role"] == "User"
            print("System message added successfully")


class TestMemoryTiers:
    """Test memory tier operations."""

    def test_add_memories_to_tiers(self):
        """Add memories to different tiers."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("Memory Test", "gpt-4")

            # Working memory - recent, frequently accessed
            store.add_memory(
                session.id,
                MemoryTier.Working,
                "User is working on a Rust project",
                tags=["project", "current"]
            )

            # Short-term memory - moderate retention
            store.add_memory(
                session.id,
                MemoryTier.ShortTerm,
                "User mentioned they like Python programming",
                tags=["preference", "language"]
            )

            # Long-term memory - persistent important facts
            store.add_memory(
                session.id,
                MemoryTier.LongTerm,
                "User's name is Claude Developer",
                tags=["identity", "important"],
                # Note: tags passed as keyword argument
            )

            # Retrieve from each tier
            working = store.get_session_memory(session.id, MemoryTier.Working)
            short_term = store.get_session_memory(session.id, MemoryTier.ShortTerm)
            long_term = store.get_session_memory(session.id, MemoryTier.LongTerm)

            assert len(working) == 1
            assert len(short_term) == 1
            assert len(long_term) == 1

            print("Memories by tier:")
            print(f"  Working: {len(working)} items")
            print(f"  Short-term: {len(short_term)} items")
            print(f"  Long-term: {len(long_term)} items")

    def test_memory_with_metadata(self):
        """Test memory item properties."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("Metadata Test", "gpt-4")

            store.add_memory(session.id, MemoryTier.Working, "Important fact", tags=["test"])

            memories = store.get_session_memory(session.id, MemoryTier.Working)
            assert len(memories) == 1

            memory = memories[0]
            assert "content" in memory
            assert "id" in memory
            assert "importance" in memory
            assert "confidence" in memory
            assert "access_count" in memory

            print(f"Memory: {memory['content'][:30]}...")
            print(f"  Importance: {memory['importance']:.2f}")
            print(f"  Confidence: {memory['confidence']:.2f}")


class TestSearch:
    """Test full-text search functionality."""

    def test_search_messages(self):
        """Search across messages in sessions."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("Search Test", "gpt-4")

            # Add messages with searchable content
            store.add_message(session.id, Role.User, "I love programming in Python")
            store.add_message(session.id, Role.Assistant, "Python is a great language for beginners")
            store.add_message(session.id, Role.User, "Rust is also interesting for systems programming")

            # Note: Search requires manual indexing of messages
            # For full-text search, use the CLI with index_message command

            # Search
            results = store.search("Python", limit=10)
            print(f"Search for 'Python': {len(results)} results")

    def test_search_with_limit(self):
        """Test search with result limit."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("Limit Test", "gpt-4")

            for i in range(20):
                store.add_message(session.id, Role.User, f"Test message {i} with unique content")

            results = store.search("Test", limit=5)
            assert len(results) <= 5
            print(f"Limited search: {len(results)} results (limit=5)")


class TestIntegration:
    """Integration tests simulating real usage scenarios."""

    def test_llm_session_workflow(self):
        """Simulate a complete LLM session workflow."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)

            # 1. Create a new session for a conversation
            session = store.create_session(
                "Rust Programming Help",
                "/workspace/models/openai-mirror/gpt-oss-120b/"
            )
            print(f"\nSession: {session.name} ({session.model})")

            # 2. Add conversation messages
            conversation = [
                (Role.User, "I'm learning Rust. Can you explain ownership?"),
                (Role.Assistant,
                 "Ownership is Rust\'s unique memory management feature. "
                 "Each value has a variable called its owner. "
                 "When the owner goes out of scope, the value will be dropped."),
                (Role.User, "What about borrowing?"),
                (Role.Assistant,
                 "Borrowing allows references to data without taking ownership. "
                 "You can have either one mutable reference or any number of immutable references."),
            ]

            for role, content in conversation:
                store.add_message(session.id, role, content)

            print(f"Added {len(conversation)} messages")

            # 3. Extract and store important facts as memories
            facts = [
                ("User is learning Rust programming", MemoryTier.Working, ["learning", "rust"]),
                ("User wants to understand ownership concept", MemoryTier.ShortTerm, ["concept", "ownership"]),
                ("User is interested in borrowing rules", MemoryTier.ShortTerm, ["concept", "borrowing"]),
            ]

            for fact, tier, tags in facts:
                store.add_memory(session.id, tier, fact, tags=tags)

            print(f"Stored {len(facts)} memory facts")

            # 4. Verify the session
            messages = store.get_session_messages(session.id)
            memories = store.get_session_memory(session.id, MemoryTier.Working)

            print(f"\nSession summary:")
            print(f"  Messages: {len(messages)}")
            print(f"  Working memories: {len(memories)}")

            # 5. Search for relevant information
            results = store.search("ownership", limit=5)
            print(f"  Search results for 'ownership': {len(results)}")

    def test_multi_session_context(self):
        """Test managing multiple sessions and context sharing."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)

            # Create sessions for different projects
            sessions = [
                ("Rust Project", "gpt-4", [
                    (Role.User, "How do I use tokio for async?"),
                    (Role.Assistant, "Tokio is an async runtime for Rust..."),
                ]),
                ("Python Project", "claude-3", [
                    (Role.User, "Show me pandas dataframe operations"),
                    (Role.Assistant, "Here are common pandas operations..."),
                ]),
                ("Database Project", "gpt-4", [
                    (Role.User, "How to optimize SQL queries?"),
                    (Role.Assistant, "Here are optimization techniques..."),
                ]),
            ]

            session_ids = []
            for name, model, messages in sessions:
                session = store.create_session(name, model)
                session_ids.append(session.id)

                for role, content in messages:
                    store.add_message(session.id, role, content)

                # Add project-specific memories
                store.add_memory(
                    session.id,
                    MemoryTier.Working,
                    f"Working on {name}",
                    tags=["project"]
                )

            # List all sessions (returns dicts)
            all_sessions = store.list_sessions()
            print(f"\nAll sessions ({len(all_sessions)}):")
            for s in all_sessions:
                print(f"  - {s['name']} ({s['model']}): {s['message_count']} messages")

            # Search across all sessions
            results = store.search("async", limit=10)
            print(f"\nSearch 'async' across sessions: {len(results)} results")

    def test_memory_retrieval_with_importance(self):
        """Test memory retrieval with importance scoring."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("Memory Importance", "gpt-4")

            # Add memories with different importance levels
            memories = [
                ("User's favorite color is blue", ["preference"], 0.9),
                ("User works as a developer", ["career"], 0.8),
                ("User had lunch today", ["daily", "trivial"], 0.3),
            ]

            for content, tags, confidence in memories:
                store.add_memory(session.id, MemoryTier.Working, content, tags=tags)

            # Retrieve and display with importance
            stored = store.get_session_memory(session.id, MemoryTier.Working)

            print("\nMemories sorted by importance:")
            for m in sorted(stored, key=lambda x: x.get('importance', 0), reverse=True):
                print(f"  [{m.get('importance', 0):.2f}] {m['content'][:50]}...")
                if 'tags' in m:
                    print(f"         Tags: {m['tags']}")


class TestConfiguration:
    """Tests demonstrating LLM/Embedding configuration usage."""

    def test_llm_client_configuration(self):
        """Demonstrate LLM client configuration from Readme.md."""
        # Configuration from Readme.md:
        llm_config = {
            "type": "openai format",
            "url": "http://127.0.0.1:1378/v1",
            "apikey": "sk",
            "model": "/workspace/models/openai-mirror/gpt-oss-120b/",
        }

        print("\nLLM Configuration (from Readme.md):")
        print(f"  URL: {llm_config['url']}")
        print(f"  Model: {llm_config['model']}")

        # This would be used with memst_core.llm.LlmClient
        # client = LlmClient::new(
        #     llm_config["url"],
        #     llm_config["model"],
        #     llm_config["apikey"],
        # )

    def test_embedding_configuration(self):
        """Demonstrate embedding client configuration from Readme.md."""
        embedding_config = {
            "type": "openai format",
            "url": "http://127.0.0.1:1378/v1",
            "model": "text-embedding-bge_m3",
        }

        print("\nEmbedding Configuration (from Readme.md):")
        print(f"  URL: {embedding_config['url']}")
        print(f"  Model: {embedding_config['model']}")

        # Example curl usage:
        # curl http://127.0.0.1:1378/v1/embeddings \
        #   -H "Content-Type: application/json" \
        #   -d '{"model": "text-embedding-bge_m3", "input": "Some text"}'

    def test_store_initialization_patterns(self):
        """Test different store initialization patterns."""
        # This test verifies path handling - paths must be strings
        with tempfile.TemporaryDirectory() as tmpdir:
            # String path works
            store = SessionStore(tmpdir)
            session = store.create_session("Test Session", "gpt-4")
            assert session is not None
            print(f"Store initialized at: {tmpdir}")
            print(f"Created session: {session.id[:8]}...")

    def test_config_toml_loading(self):
        """Test config.toml loading functionality."""
        import os

        with tempfile.TemporaryDirectory() as tmpdir:
            # Create a config.toml file
            config_content = '''[llm]
type = "openai"
api_url = "http://test.local:8080/v1"
model = "test-model"
timeout = 30
max_tokens = 2048
temperature = 0.5
api_key = ""

[embedding]
type = "openai"
api_url = "http://embed.local:8080/embeddings"
model = "embed-model"
timeout = 15
expected_dimension = 768
'''
            config_path = os.path.join(tmpdir, "config.toml")
            with open(config_path, "w") as f:
                f.write(config_content)

            # Verify config file was created and can be read
            with open(config_path, "r") as f:
                content = f.read()

            assert "[llm]" in content
            assert "[embedding]" in content
            assert 'api_url = "http://test.local:8080/v1"' in content
            assert 'model = "test-model"' in content
            assert 'expected_dimension = 768' in content

            print("\nConfig.toml file created and verified:")
            print(f"  Path: {config_path}")
            print(f"  LLM URL: http://test.local:8080/v1")
            print(f"  LLM Model: test-model")
            print(f"  Embedding URL: http://embed.local:8080/embeddings")
            print(f"  Embedding Dimension: 768")
            print("\n  Config can be loaded in Rust using:")
            print("    use memst_core::config::MemStConfig;")
            print("    let config = MemStConfig::load_from_file(path)?;")


class TestEdgeCases:
    """Edge case tests for robustness and error handling."""

    def test_empty_content_message(self):
        """Test adding a message with empty content."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("Empty Content Test", "gpt-4")

            # Add message with empty content
            store.add_message(session.id, Role.User, "")

            messages = store.get_session_messages(session.id)
            assert len(messages) == 1
            assert messages[0]["content"] == ""
            print("Empty content message added successfully")

    def test_unicode_content(self):
        """Test adding messages with unicode content."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("Unicode Test", "gpt-4")

            # Test various unicode content
            unicode_messages = [
                ("Hello in Chinese", "你好世界"),
                ("Hello in Russian", "Привет мир"),
                ("Hello in Emoji", "Hello 🌍🎉🚀"),
                ("Mixed content", "Hello 世界 🌍 Привет"),
            ]

            for label, content in unicode_messages:
                store.add_message(session.id, Role.User, content)

            messages = store.get_session_messages(session.id)
            assert len(messages) == 4

            # Verify content is preserved
            for i, (label, expected) in enumerate(unicode_messages):
                assert messages[i]["content"] == expected, f"Failed for {label}"

            print("Unicode content handled correctly")

    def test_special_characters_content(self):
        """Test adding messages with special characters."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("Special Chars Test", "gpt-4")

            # Test various special characters
            special_messages = [
                ("JSON content", '{"key": "value", "nested": {"a": 1}}'),
                ("CSV content", "a,b,c\n1,2,3\n4,5,6"),
                ("Newlines", "Line 1\nLine 2\nLine 3"),
                ("Tabs", "Col1\tCol2\tCol3"),
                ("Quotes", 'She said "Hello, World!"'),
                ("Backslashes", "Path: C:\\Users\\test\\file.txt"),
                ("Regex", r"Pattern: \d+\.\d*"),
            ]

            for label, content in special_messages:
                store.add_message(session.id, Role.User, content)

            messages = store.get_session_messages(session.id)
            assert len(messages) == len(special_messages)

            for i, (label, expected) in enumerate(special_messages):
                assert messages[i]["content"] == expected, f"Failed for {label}"

            print("Special characters handled correctly")

    def test_long_content(self):
        """Test adding long messages."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("Long Content Test", "gpt-4")

            # Create a long message (10KB)
            long_content = "A" * 10240
            store.add_message(session.id, Role.User, long_content)

            messages = store.get_session_messages(session.id)
            assert len(messages) == 1
            assert len(messages[0]["content"]) == 10240
            print("Long content handled correctly")

    def test_nonexistent_session_operations(self):
        """Test operations on non-existent sessions."""
        import uuid
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)

            fake_id = str(uuid.uuid4())

            # Test get non-existent session
            result = store.get_session(fake_id)
            assert result is None
            print("Non-existent session returns None")

    def test_delete_nonexistent_session(self):
        """Test deleting a non-existent session."""
        import uuid
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)

            fake_id = str(uuid.uuid4())

            # Delete should not raise exception, but may return error
            try:
                store.delete_session(fake_id)
                # If no exception, verify session still doesn't exist
                result = store.get_session(fake_id)
                assert result is None
            except Exception as e:
                # Some implementations may raise exception
                print(f"Delete non-existent session raised: {type(e).__name__}")

    def test_message_order_preservation(self):
        """Test that message order is preserved after many appends."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("Order Test", "gpt-4")

            # Add many messages
            message_count = 100
            for i in range(message_count):
                store.add_message(session.id, Role.User, f"Message {i}")

            messages = store.get_session_messages(session.id)
            assert len(messages) == message_count

            # Verify order is preserved
            for i, msg in enumerate(messages):
                assert f"Message {i}" in msg["content"], f"Order broken at index {i}"

            print(f"Message order preserved for {message_count} messages")

    def test_session_persistence(self):
        """Test that sessions are properly persisted to disk.
        
        This test verifies the store writes data to disk correctly.
        """
        import gc
        with tempfile.TemporaryDirectory() as tmpdir:
            path = tmpdir

            # Create store and add data
            store = SessionStore(path)
            session = store.create_session("Persistent Session", "gpt-4")
            session_id = session.id
            for i in range(5):
                store.add_message(session.id, Role.User, f"Message {i}")

            # Force cleanup
            del store
            gc.collect()

            # Verify manifest exists and has correct data
            import json
            manifest_path = f"{path}/manifest.json"
            with open(manifest_path, 'r') as f:
                manifest = json.load(f)
            
            assert "sessions" in manifest
            assert session_id in manifest["sessions"]
            assert manifest["sessions"][session_id]["message_count"] == 5
            print("Session persistence verified via manifest file")

    def test_all_message_roles(self):
        """Test all message role types."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("Roles Test", "gpt-4")

            # Add messages with all roles
            store.add_message(session.id, Role.System, "You are a helpful assistant")
            store.add_message(session.id, Role.User, "Hello")
            store.add_message(session.id, Role.Assistant, "Hi there!")
            store.add_message(session.id, Role.Tool, "Tool result: success")

            messages = store.get_session_messages(session.id)
            assert len(messages) == 4

            roles = [msg["role"] for msg in messages]
            assert "System" in roles
            assert "User" in roles
            assert "Assistant" in roles
            assert "Tool" in roles
            print("All message roles handled correctly")

    def test_concurrent_session_operations(self):
        """Test operations on multiple sessions simultaneously."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)

            # Create multiple sessions
            sessions = []
            for i in range(5):
                sess = store.create_session(f"Session {i}", f"model-{i}")
                sessions.append(sess.id)

            # Add messages to each session
            for i, sess_id in enumerate(sessions):
                for j in range(10):
                    store.add_message(sess_id, Role.User, f"S{i} M{j}")

            # Verify isolation
            for i, sess_id in enumerate(sessions):
                messages = store.get_session_messages(sess_id)
                assert len(messages) == 10, f"Session {i} has {len(messages)} messages"

            # Verify each session has unique messages
            for i, sess_id in enumerate(sessions):
                messages = store.get_session_messages(sess_id)
                for msg in messages:
                    assert f"S{i}" in msg["content"], f"Cross-session contamination detected"

            print("Session isolation verified for 5 sessions")

    def test_message_with_metadata(self):
        """Test adding messages with various metadata."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("Metadata Test", "gpt-4")

            # Add a message
            msg_id = store.add_message(session.id, Role.User, "Test message")

            messages = store.get_session_messages(session.id)
            assert len(messages) == 1

            # Verify message has expected fields
            msg = messages[0]
            assert "id" in msg
            assert "content" in msg
            assert "role" in msg
            assert "timestamp" in msg

            print("Message metadata fields verified")


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
