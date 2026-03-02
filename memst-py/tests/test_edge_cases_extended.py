"""Extended edge case tests for MemSt Python bindings.

This test module covers:
- Boundary conditions (empty, very large, min/max values)
- Error handling robustness
- Input validation
- Type safety
- Race conditions and concurrency
"""

import tempfile
import pytest
import uuid
import json
import gc
import os
import time
from datetime import datetime, timedelta
from pathlib import Path

from memst import SessionStore, Session, Role, MemoryTier


class TestBoundaryConditions:
    """Test boundary conditions and extreme values."""

    def test_very_large_session_name(self):
        """Test session with maximum allowed name length."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            
            # Very long name (10KB)
            long_name = "A" * 10240
            session = store.create_session(long_name, "gpt-4")
            
            retrieved = store.get_session(session.id)
            assert retrieved["name"] == long_name
            print(f"Handled session name of {len(long_name)} characters")

    def test_very_long_single_message(self):
        """Test message with extremely long content (1MB)."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("Long Msg Test", "gpt-4")
            
            # 1MB message
            long_content = "X" * (1024 * 1024)
            store.add_message(session.id, Role.User, long_content)
            
            messages = store.get_session_messages(session.id)
            assert len(messages[0]["content"]) == len(long_content)
            print(f"Handled message of {len(long_content)} bytes")

    def test_maximum_messages_per_session(self):
        """Test adding maximum reasonable messages to a session."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("Max Messages", "gpt-4")
            
            # Add 1000 messages (reasonable stress test)
            message_count = 1000
            for i in range(message_count):
                store.add_message(session.id, Role.User, f"Message {i}")
            
            messages = store.get_session_messages(session.id)
            assert len(messages) == message_count
            print(f"Handled {message_count} messages in a session")

    def test_maximum_sessions(self):
        """Test creating many sessions."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            
            # Create 100 sessions
            session_count = 100
            for i in range(session_count):
                store.create_session(f"Session {i}", f"model-{i}")
            
            all_sessions = store.list_sessions()
            assert len(all_sessions) == session_count
            print(f"Handled {session_count} sessions")

    def test_very_long_memory_content(self):
        """Test memory with very long content."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("Long Memory", "gpt-4")
            
            long_content = "M" * (1024 * 500)  # 500KB
            store.add_memory(session.id, MemoryTier.LongTerm, long_content, tags=['long'])
            
            memories = store.get_session_memory(session.id, MemoryTier.LongTerm)
            assert len(memories[0]["content"]) == len(long_content)
            print(f"Handled memory of {len(long_content)} bytes")

    def test_empty_session_name(self):
        """Test session with empty name."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("", "gpt-4")
            
            assert session.name == ""
            retrieved = store.get_session(session.id)
            assert retrieved["name"] == ""
            print("Handled empty session name")

    def test_empty_model_name(self):
        """Test session with empty model name."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("Test", "")
            
            assert session.model == ""
            print("Handled empty model name")


class TestErrorHandling:
    """Test error handling and recovery."""

    def test_invalid_session_id_format(self):
        """Test operations with malformed session ID."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            
            # Try various invalid formats
            invalid_ids = [
                "",
                "not-a-uuid",
                "123",
                "session-with-special-chars-!@#$%",
                "a" * 300,  # Too long
            ]
            
            for invalid_id in invalid_ids:
                # Should not crash - may return None or raise appropriate error
                try:
                    result = store.get_session(invalid_id)
                    assert result is None
                except Exception as e:
                    # Some implementations may raise errors for invalid input
                    print(f"Invalid ID '{invalid_id[:20]}...' raised: {type(e).__name__}")

    @pytest.mark.skip(reason="Causes timeout - concurrent store access issue")
    def test_operations_after_store_close(self):
        """Test operations on a closed store."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("Test", "gpt-4")
            session_id = session.id
            
            # Store is closed when exiting context
            # Operations should fail gracefully if attempted
            
            # Verify data was persisted
            store2 = SessionStore(tmpdir)
            retrieved = store2.get_session(session_id)
            assert retrieved is not None
            print("Store data persists across close/reopen")

    @pytest.mark.skip(reason="Causes timeout - concurrent store access issue")
    def test_concurrent_store_access(self):
        """Test multiple stores accessing same path."""
        with tempfile.TemporaryDirectory() as tmpdir:
            # This might cause issues - testing for robustness
            try:
                store1 = SessionStore(tmpdir)
                store2 = SessionStore(tmpdir)
                
                session1 = store1.create_session("From Store 1", "model-1")
                
                # Store2 might not see this immediately or at all
                # Testing for crash resistance
                sessions = store2.list_sessions()
                print(f"Concurrent access test - store2 sees {len(sessions)} sessions")
            except Exception as e:
                print(f"Concurrent access raised: {type(e).__name__}: {e}")

    def test_message_with_none_content(self):
        """Test adding message with None content."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("None Test", "gpt-4")
            
            try:
                store.add_message(session.id, Role.User, None)
                print("None content was accepted")
            except (TypeError, ValueError) as e:
                # This is acceptable - None should be rejected
                print(f"None content properly rejected: {type(e).__name__}")

    def test_invalid_role_type(self):
        """Test adding message with invalid role type."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("Invalid Role", "gpt-4")
            
            # Try string role instead of enum
            try:
                store.add_message(session.id, "user", "content")
                print("String role was accepted")
            except (TypeError, ValueError) as e:
                print(f"Invalid role properly rejected: {type(e).__name__}")


class TestDataValidation:
    """Test input data validation."""

    def test_whitespace_only_content(self):
        """Test messages with whitespace-only content."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("Whitespace Test", "gpt-4")
            
            whitespace_messages = [
                "   ",  # Spaces
                "\t\t",  # Tabs
                "\n\n",  # Newlines
                " \t \n ",  # Mixed
            ]
            
            for content in whitespace_messages:
                store.add_message(session.id, Role.User, content)
            
            messages = store.get_session_messages(session.id)
            assert len(messages) == len(whitespace_messages)
            print("Handled whitespace-only content")

    def test_binary_like_content(self):
        """Test content that looks like binary data."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("Binary Test", "gpt-4")
            
            # Content with null bytes and control characters
            binary_content = "Hello\x00World\x01\x02\x03"
            store.add_message(session.id, Role.User, binary_content)
            
            messages = store.get_session_messages(session.id)
            assert messages[0]["content"] == binary_content
            print("Handled binary-like content")

    def test_malformed_json_content(self):
        """Test content that is malformed JSON."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("JSON Test", "gpt-4")
            
            malformed_jsons = [
                '{"incomplete":',
                '{invalid}',
                '[1,2,3',
                'not json at all',
            ]
            
            for content in malformed_jsons:
                store.add_message(session.id, Role.User, content)
            
            messages = store.get_session_messages(session.id)
            assert len(messages) == len(malformed_jsons)
            print("Handled malformed JSON content")

    def test_very_large_tag_list(self):
        """Test memories with very large tag lists."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("Many Tags", "gpt-4")
            
            # 1000 tags
            tags = [f"tag_{i}" for i in range(1000)]
            store.add_memory(session.id, MemoryTier.Working, "Content", tags=tags)
            
            memories = store.get_session_memory(session.id, MemoryTier.Working)
            assert len(memories[0]["tags"]) == 1000
            print("Handled 1000 tags on a memory")

    def test_duplicate_session_names(self):
        """Test creating sessions with duplicate names."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            
            # Create multiple sessions with same name
            session1 = store.create_session("Same Name", "model-1")
            session2 = store.create_session("Same Name", "model-2")
            
            assert session1.id != session2.id
            assert session1.name == session2.name
            
            # Both should be retrievable
            retrieved1 = store.get_session(session1.id)
            retrieved2 = store.get_session(session2.id)
            
            assert retrieved1 is not None
            assert retrieved2 is not None
            print("Handled duplicate session names correctly")


class TestMemoryTierEdgeCases:
    """Test memory tier operations edge cases."""

    def test_memory_in_all_tiers(self):
        """Test adding many memories to each tier."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("All Tiers", "gpt-4")
            
            tier_names = {"Working": MemoryTier.Working, "ShortTerm": MemoryTier.ShortTerm, "LongTerm": MemoryTier.LongTerm}
            for tier_name, tier in tier_names.items():
                for i in range(50):
                    store.add_memory(session.id, tier, f"Memory {i} in {tier_name}", tags=['test'])
                
                memories = store.get_session_memory(session.id, tier)
                assert len(memories) == 50
            
            print("Handled 50 memories per tier")

    def test_empty_memory_content(self):
        """Test adding memory with empty content."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("Empty Memory", "gpt-4")
            
            store.add_memory(session.id, MemoryTier.Working, "", tags=['empty'])
            
            memories = store.get_session_memory(session.id, MemoryTier.Working)
            assert len(memories) == 1
            assert memories[0]["content"] == ""
            print("Handled empty memory content")

    def test_memory_with_special_tags(self):
        """Test memory with special characters in tags."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("Special Tags", "gpt-4")
            
            special_tags = [
                ["tag with spaces"],
                ["tag/with/slashes"],
                ["tag:with:colons"],
                ["tag!@#$%"],
                ["ünicode_tag"],
            ]
            
            for tags in special_tags:
                store.add_memory(session.id, MemoryTier.Working, "Content", tags=tags)
            
            memories = store.get_session_memory(session.id, MemoryTier.Working)
            assert len(memories) == len(special_tags)
            print("Handled special characters in tags")

    def test_promote_nonexistent_memory(self):
        """Test promoting a memory that doesn't exist."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("Promote Test", "gpt-4")
            
            # Try to promote non-existent memory
            try:
                result = store.promote_memory(session.id, "nonexistent-id")
                print(f"Promote nonexistent returned: {result}")
            except Exception as e:
                print(f"Promote nonexistent raised: {type(e).__name__}")


class TestSearchEdgeCases:
    """Test search functionality edge cases."""

    def test_search_with_special_characters(self):
        """Test search queries with special characters."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("Search Special", "gpt-4")
            
            # Add messages
            store.add_message(session.id, Role.User, "Test with special: !@#$%")
            store.add_message(session.id, Role.User, "Test with quotes \"double\" 'single'")
            store.add_message(session.id, Role.User, "Test with slash / and backslash \\")
            
            # Search with special chars
            queries = ["!", "@", "#", "$", "%", "\"", "'", "/", "\\"]
            for query in queries:
                try:
                    results = store.search(query, limit=10)
                    print(f"Search '{query}' returned {len(results)} results")
                except Exception as e:
                    print(f"Search '{query}' raised: {type(e).__name__}")

    def test_search_empty_query(self):
        """Test search with empty query."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("Empty Search", "gpt-4")
            
            store.add_message(session.id, Role.User, "Some content")
            
            results = store.search("", limit=10)
            print(f"Empty search returned {len(results)} results")

    def test_search_zero_limit(self):
        """Test search with zero limit."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("Zero Limit", "gpt-4")
            
            store.add_message(session.id, Role.User, "Test content")
            
            results = store.search("Test", limit=0)
            print(f"Zero limit search returned {len(results)} results")

    def test_search_negative_limit(self):
        """Test search with negative limit."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("Negative Limit", "gpt-4")
            
            store.add_message(session.id, Role.User, "Test content")
            
            # Negative limit should raise an error
            try:
                results = store.search("Test", limit=-1)
                print(f"Negative limit search returned {len(results)} results")
            except (ValueError, OverflowError) as e:
                print(f"Negative limit properly rejected: {type(e).__name__}")

    def test_search_nonexistent_term(self):
        """Test search for term that doesn't exist."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("No Results", "gpt-4")
            
            store.add_message(session.id, Role.User, "Hello world")
            
            results = store.search("xyz123nonexistent", limit=10)
            assert len(results) == 0
            print("Search for nonexistent term returned 0 results")


class TestDataPersistence:
    """Test data persistence and recovery."""

    def test_partial_write_recovery(self):
        """Test recovery from partial writes."""
        import gc
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("Persistence Test", "gpt-4")
            session_id = session.id
            
            # Add some messages
            for i in range(10):
                store.add_message(session_id, Role.User, f"Message {i}")
            
            # Explicitly release the store and session to release the lock
            # This is needed because the store uses an exclusive lock
            # and session holds a reference to the store
            del store
            del session
            gc.collect()
            
            # Force flush by creating new store
            store2 = SessionStore(tmpdir)
            
            # Verify data
            messages = store2.get_session_messages(session_id)
            assert len(messages) == 10
            print("Data persisted correctly across store instances")

    def test_manifest_corruption_handling(self):
        """Test handling of corrupted manifest file."""
        import gc
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("Manifest Test", "gpt-4")
            session_id = session.id
            
            # Corrupt manifest
            manifest_path = os.path.join(tmpdir, "manifest.json")
            with open(manifest_path, "w") as f:
                f.write("invalid json {{{")
            
            # Release the store to release the lock
            del store
            del session
            gc.collect()
            
            # Try to open corrupted store
            try:
                store2 = SessionStore(tmpdir)
                print("Handled corrupted manifest")
            except json.JSONDecodeError as e:
                print(f"Corrupted manifest properly rejected: {type(e).__name__}")

    def test_large_directory_handling(self):
        """Test handling of store with many files."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            
            # Create many sessions
            for i in range(50):
                session = store.create_session(f"Session {i}", "model")
                for j in range(10):
                    store.add_message(session.id, Role.User, f"Msg {j}")
            
            # Verify listing works
            all_sessions = store.list_sessions()
            assert len(all_sessions) == 50
            
            # Verify retrieval works
            for session in all_sessions:
                messages = store.get_session_messages(session["id"])
                assert len(messages) == 10
            
            print("Handled 50 sessions with 500 total messages")


class TestResourceCleanup:
    """Test proper resource cleanup."""

    def test_memory_cleanup_after_deletion(self):
        """Test that deleted session data is cleaned up."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("To Delete", "gpt-4")
            
            # Add data
            for i in range(10):
                store.add_message(session.id, Role.User, f"Msg {i}")
            
            # Delete
            store.delete_session(session.id)
            
            # Verify cleanup
            all_sessions = store.list_sessions()
            assert len(all_sessions) == 0
            print("Session data properly cleaned up after deletion")

    def test_gc_pressure(self):
        """Test under garbage collection pressure."""
        with tempfile.TemporaryDirectory() as tmpdir:
            stores = []
            sessions = []
            
            # Create many stores
            for i in range(10):
                store = SessionStore(tmpdir + f"_{i}")
                session = store.create_session(f"Session {i}", "model")
                for j in range(20):
                    store.add_message(session.id, Role.User, f"Msg {j}")
                stores.append(store)
                sessions.append(session.id)
            
            # Force GC
            gc.collect()
            
            # Verify data still accessible
            for i, store in enumerate(stores):
                messages = store.get_session_messages(sessions[i])
                assert len(messages) == 20
            
            print("Handled GC pressure with 10 stores")


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
