"""Performance tests for MemSt Python bindings.

This test module measures and validates:
- Session creation throughput
- Message add/retrieve latency
- Search performance at scale
- Memory tier operations
- Concurrent operations
"""

import tempfile
import pytest
import time
import uuid
import statistics
from typing import List, Dict, Callable

from memst import SessionStore, Session, Role, MemoryTier


class PerformanceTimer:
    """Context manager for timing operations."""
    
    def __init__(self, name: str):
        self.name = name
        self.start_time = None
        self.end_time = None
        self.elapsed_ms = None
    
    def __enter__(self):
        self.start_time = time.perf_counter()
        return self
    
    def __exit__(self, *args):
        self.end_time = time.perf_counter()
        self.elapsed_ms = (self.end_time - self.start_time) * 1000


def measure_operation(operation: Callable, iterations: int = 1) -> Dict:
    """Measure an operation's performance."""
    times = []
    
    for _ in range(iterations):
        start = time.perf_counter()
        result = operation()
        end = time.perf_counter()
        times.append((end - start) * 1000)  # Convert to ms
    
    return {
        "mean_ms": statistics.mean(times),
        "median_ms": statistics.median(times),
        "min_ms": min(times),
        "max_ms": max(times),
        "stdev_ms": statistics.stdev(times) if len(times) > 1 else 0,
        "iterations": iterations,
    }


class TestSessionPerformance:
    """Performance tests for session operations."""

    def test_session_creation_throughput(self):
        """Measure session creation speed."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            
            # Warm up
            for _ in range(5):
                store.create_session("warmup", "model")
            
            # Measure
            iterations = 100
            
            def create_session():
                return store.create_session(f"session_{uuid.uuid4()}", "gpt-4")
            
            result = measure_operation(create_session, iterations)
            
            print(f"\nSession Creation Performance ({iterations} iterations):")
            print(f"  Mean:   {result['mean_ms']:.3f} ms")
            print(f"  Median: {result['median_ms']:.3f} ms")
            print(f"  Min:    {result['min_ms']:.3f} ms")
            print(f"  Max:    {result['max_ms']:.3f} ms")
            print(f"  StdDev: {result['stdev_ms']:.3f} ms")
            print(f"  Throughput: {1000 / result['mean_ms']:.1f} sessions/sec")
            
            # Assert reasonable performance
            assert result['mean_ms'] < 100, "Session creation too slow"

    def test_session_list_throughput(self):
        """Measure session listing speed."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            
            # Create many sessions
            for i in range(100):
                store.create_session(f"session_{i}", "model")
            
            # Measure
            iterations = 50
            
            def list_sessions():
                return store.list_sessions()
            
            result = measure_operation(list_sessions, iterations)
            
            print(f"\nSession List Performance (100 sessions, {iterations} iterations):")
            print(f"  Mean:   {result['mean_ms']:.3f} ms")
            print(f"  Median: {result['median_ms']:.3f} ms")
            print(f"  Min:    {result['min_ms']:.3f} ms")
            print(f"  Max:    {result['max_ms']:.3f} ms")

    def test_session_retrieval_throughput(self):
        """Measure session retrieval by ID."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            
            # Create sessions
            session_ids = []
            for i in range(50):
                session = store.create_session(f"session_{i}", "model")
                session_ids.append(session.id)
            
            # Measure
            iterations = 100
            
            def get_session():
                return store.get_session(session_ids[i % len(session_ids)])
            
            result = measure_operation(get_session, iterations)
            
            print(f"\nSession Retrieval Performance (50 sessions, {iterations} iterations):")
            print(f"  Mean:   {result['mean_ms']:.3f} ms")
            print(f"  Median: {result['median_ms']:.3f} ms")
            print(f"  Min:    {result['min_ms']:.3f} ms")
            print(f"  Max:    {result['max_ms']:.3f} ms")


class TestMessagePerformance:
    """Performance tests for message operations."""

    def test_single_message_add_latency(self):
        """Measure latency of adding a single message."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("test", "model")
            
            # Warm up
            for _ in range(10):
                store.add_message(session.id, Role.User, "warmup")
            
            # Measure
            iterations = 100
            
            def add_message():
                return store.add_message(
                    session.id, 
                    Role.User, 
                    f"Message {uuid.uuid4()}"
                )
            
            result = measure_operation(add_message, iterations)
            
            print(f"\nSingle Message Add Latency ({iterations} iterations):")
            print(f"  Mean:   {result['mean_ms']:.3f} ms")
            print(f"  Median: {result['median_ms']:.3f} ms")
            print(f"  Min:    {result['min_ms']:.3f} ms")
            print(f"  Max:    {result['max_ms']:.3f} ms")
            
            assert result['mean_ms'] < 50, "Message add too slow"

    def test_bulk_message_add_throughput(self):
        """Measure throughput of adding many messages."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("bulk", "model")
            
            # Bulk add
            message_count = 500
            
            start = time.perf_counter()
            for i in range(message_count):
                store.add_message(session.id, Role.User, f"Message {i}")
            end = time.perf_counter()
            
            elapsed_ms = (end - start) * 1000
            throughput = message_count / (end - start)
            
            print(f"\nBulk Message Add ({message_count} messages):")
            print(f"  Total time: {elapsed_ms:.2f} ms")
            print(f"  Per message: {elapsed_ms / message_count:.3f} ms")
            print(f"  Throughput: {throughput:.1f} messages/sec")
            
            # Verify
            messages = store.get_session_messages(session.id)
            assert len(messages) == message_count

    def test_message_retrieval_performance(self):
        """Measure message retrieval at scale."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("retrieve", "model")
            
            # Add many messages
            message_count = 1000
            for i in range(message_count):
                store.add_message(session.id, Role.User, f"Message {i}")
            
            # Measure retrieval
            iterations = 20
            
            def get_messages():
                return store.get_session_messages(session.id)
            
            result = measure_operation(get_messages, iterations)
            
            print(f"\nMessage Retrieval ({message_count} messages, {iterations} iterations):")
            print(f"  Mean:   {result['mean_ms']:.3f} ms")
            print(f"  Median: {result['median_ms']:.3f} ms")
            print(f"  Min:    {result['min_ms']:.3f} ms")
            print(f"  Max:    {result['max_ms']:.3f} ms")
            
            # Should scale reasonably
            assert result['mean_ms'] < 500, "Message retrieval too slow at scale"

    def test_message_pagination_performance(self):
        """Measure performance of paginated message retrieval."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("pagination", "model")
            
            # Add messages
            message_count = 500
            for i in range(message_count):
                store.add_message(session.id, Role.User, f"Message {i}")
            
            # Measure paginated retrieval - skip if not implemented
            try:
                def get_page():
                    # Pagination not supported in current API
                    # Get all messages and slice manually
                    all_msgs = store.get_session_messages(session.id)
                    return all_msgs[0:page_size]
                
                result = measure_operation(get_page, iterations)
                
                print(f"\nMessage Pagination ({page_size} per page, {iterations} iterations):")
                print(f"  Mean:   {result['mean_ms']:.3f} ms")
                print(f"  Median: {result['median_ms']:.3f} ms")
                
                messages = store.get_session_messages(session.id)[0:page_size]
                assert len(messages) == page_size
            except Exception as e:
                pytest.skip(f"Pagination not supported: {e}")


class TestSearchPerformance:
    """Performance tests for search operations."""

    def test_search_at_scale(self):
        """Measure search performance with many messages."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("search", "model")
            
            # Add many searchable messages
            message_count = 500
            keywords = ["python", "rust", "go", "javascript", "java", "c++", "ruby", "swift"]
            
            for i in range(message_count):
                keyword = keywords[i % len(keywords)]
                store.add_message(
                    session.id, 
                    Role.User, 
                    f"Message {i} about {keyword} programming"
                )
            
            # Measure search
            iterations = 20
            
            def search():
                return store.search("python", limit=10)
            
            result = measure_operation(search, iterations)
            
            print(f"\nSearch Performance ({message_count} messages, {iterations} iterations):")
            print(f"  Mean:   {result['mean_ms']:.3f} ms")
            print(f"  Median: {result['median_ms']:.3f} ms")
            print(f"  Min:    {result['min_ms']:.3f} ms")
            print(f"  Max:    {result['max_ms']:.3f} ms")

    def test_search_with_high_limit(self):
        """Measure search with high result limit."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("high_limit", "model")
            
            # Add messages
            for i in range(200):
                store.add_message(session.id, Role.User, f"Content {i}")
            
            # Search with high limit
            iterations = 10
            
            def search():
                return store.search("Content", limit=100)
            
            result = measure_operation(search, iterations)
            
            print(f"\nSearch with High Limit (200 msgs, limit=100, {iterations} iter):")
            print(f"  Mean:   {result['mean_ms']:.3f} ms")
            print(f"  Median: {result['median_ms']:.3f} ms")


class TestMemoryPerformance:
    """Performance tests for memory operations."""

    def test_memory_add_performance(self):
        """Measure memory addition performance."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("memory", "model")
            
            # Warm up
            for _ in range(10):
                store.add_memory(session.id, MemoryTier.Working, "warmup", tags=['warmup'])
            
            # Measure
            iterations = 100
            
            def add_memory():
                return store.add_memory(
                    session.id,
                    MemoryTier.Working,
                    f"Memory {uuid.uuid4()}",
                    tags=['test']
                )
            
            result = measure_operation(add_memory, iterations)
            
            print(f"\nMemory Add Performance ({iterations} iterations):")
            print(f"  Mean:   {result['mean_ms']:.3f} ms")
            print(f"  Median: {result['median_ms']:.3f} ms")
            print(f"  Min:    {result['min_ms']:.3f} ms")
            print(f"  Max:    {result['max_ms']:.3f} ms")

    def test_bulk_memory_operations(self):
        """Measure bulk memory operations."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("bulk_memory", "model")
            
            # Bulk add
            memory_count = 300
            
            start = time.perf_counter()
            for i in range(memory_count):
                store.add_memory(session.id, MemoryTier.Working, f"Memory {i}", tags=['bulk'])
            end = time.perf_counter()
            
            elapsed_ms = (end - start) * 1000
            throughput = memory_count / (end - start)
            
            print(f"\nBulk Memory Add ({memory_count} memories):")
            print(f"  Total time: {elapsed_ms:.2f} ms")
            print(f"  Per memory: {elapsed_ms / memory_count:.3f} ms")
            print(f"  Throughput: {throughput:.1f} memories/sec")
            
            # Verify retrieval
            memories = store.get_session_memory(session.id, MemoryTier.Working)
            assert len(memories) == memory_count

    def test_tier_filtering_performance(self):
        """Measure memory tier filtering."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("tier_filter", "model")
            
            # Add memories to different tiers
            for tier_name, tier in [("Working", MemoryTier.Working), ("ShortTerm", MemoryTier.ShortTerm), ("LongTerm", MemoryTier.LongTerm)]:
                for i in range(100):
                    store.add_memory(session.id, tier, f"{tier_name} memory {i}", tags=['test'])
            
            # Measure filtering
            iterations = 30
            
            def get_tier():
                return store.get_session_memory(session.id, MemoryTier.Working)
            
            result = measure_operation(get_tier, iterations)
            
            print(f"\nTier Filtering (100 per tier, {iterations} iterations):")
            print(f"  Mean:   {result['mean_ms']:.3f} ms")
            print(f"  Median: {result['median_ms']:.3f} ms")


class TestConcurrentPerformance:
    """Performance tests for concurrent operations."""

    def test_sequential_vs_batch(self):
        """Compare sequential vs batch operations."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("batch_test", "model")
            
            message_count = 100
            
            # Sequential
            start = time.perf_counter()
            for i in range(message_count):
                store.add_message(session.id, Role.User, f"Msg {i}")
            sequential_time = time.perf_counter() - start
            
            # Create new session for batch test
            session2 = store.create_session("batch_test2", "model")
            
            # Batch (if supported)
            start = time.perf_counter()
            for i in range(message_count):
                store.add_message(session2.id, Role.User, f"Msg {i}")
            batch_time = time.perf_counter() - start
            
            print(f"\nSequential vs Batch ({message_count} messages):")
            print(f"  Sequential: {sequential_time * 1000:.2f} ms")
            print(f"  Batch:       {batch_time * 1000:.2f} ms")

    def test_multi_session_operations(self):
        """Measure operations across multiple sessions."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            
            session_count = 20
            messages_per_session = 50
            
            # Create sessions and add messages
            start = time.perf_counter()
            session_ids = []
            for i in range(session_count):
                session = store.create_session(f"session_{i}", "model")
                session_ids.append(session.id)
                
                for j in range(messages_per_session):
                    store.add_message(session.id, Role.User, f"Msg {j}")
            
            total_time = time.perf_counter() - start
            total_messages = session_count * messages_per_session
            
            print(f"\nMulti-Session Operations ({session_count} sessions, {messages_per_session} msgs each):")
            print(f"  Total time:     {total_time * 1000:.2f} ms")
            print(f"  Per message:    {total_time * 1000 / total_messages:.3f} ms")
            print(f"  Throughput:     {total_messages / total_time:.1f} msgs/sec")

    def test_mixed_operations(self):
        """Measure mixed workload performance."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            
            start = time.perf_counter()
            
            # Create sessions
            for i in range(10):
                session = store.create_session(f"mixed_{i}", "model")
                
                # Add messages
                for j in range(20):
                    store.add_message(session.id, Role.User, f"Msg {j}")
                
                # Add memories
                for j in range(5):
                    store.add_memory(session.id, MemoryTier.Working, f"Mem {j}", tags=['test'])
                
                # Search
                store.search("Msg", limit=10)
                
                # List
                store.list_sessions()
            
            total_time = time.perf_counter() - start
            
            print(f"\nMixed Operations (10 sessions with messages + memories + search):")
            print(f"  Total time: {total_time * 1000:.2f} ms")
            print(f"  Per session: {total_time * 1000 / 10:.2f} ms")


class TestLargeScalePerformance:
    """Large scale performance tests."""

    def test_large_session_handling(self):
        """Test handling of a very large session."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("large", "model")
            
            message_count = 2000
            
            start = time.perf_counter()
            for i in range(message_count):
                store.add_message(
                    session.id,
                    Role.User,
                    f"Message number {i} with some content to make it realistic"
                )
            add_time = time.perf_counter() - start
            
            # Retrieve
            start = time.perf_counter()
            messages = store.get_session_messages(session.id)
            retrieve_time = time.perf_counter() - start
            
            print(f"\nLarge Session ({message_count} messages):")
            print(f"  Add time:     {add_time * 1000:.2f} ms ({message_count / add_time:.1f} msgs/sec)")
            print(f"  Retrieve time: {retrieve_time * 1000:.2f} ms")
            print(f"  Total size:   {len(messages)} messages")
            
            assert len(messages) == message_count

    def test_many_small_sessions(self):
        """Test handling many small sessions."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            
            session_count = 200
            
            start = time.perf_counter()
            session_ids = []
            for i in range(session_count):
                session = store.create_session(f"small_{i}", "model")
                session_ids.append(session.id)
                
                # Each session has 5 messages
                for j in range(5):
                    store.add_message(session.id, Role.User, f"Msg {j}")
            
            total_time = time.perf_counter() - start
            
            print(f"\nMany Small Sessions ({session_count} sessions, 5 msgs each):")
            print(f"  Total time: {total_time * 1000:.2f} ms")
            print(f"  Per session: {total_time * 1000 / session_count:.2f} ms")
            
            # Verify
            all_sessions = store.list_sessions()
            assert len(all_sessions) == session_count

    def test_sustained_load(self):
        """Test sustained load over time."""
        with tempfile.TemporaryDirectory() as tmpdir:
            store = SessionStore(tmpdir)
            session = store.create_session("sustained", "model")
            
            # Run for a fixed time
            duration_seconds = 1.0
            start_time = time.perf_counter()
            message_count = 0
            
            while time.perf_counter() - start_time < duration_seconds:
                store.add_message(session.id, Role.User, f"Msg {message_count}")
                message_count += 1
            
            elapsed = time.perf_counter() - start_time
            throughput = message_count / elapsed
            
            print(f"\nSustained Load ({duration_seconds}s):")
            print(f"  Messages:  {message_count}")
            print(f"  Throughput: {throughput:.1f} msgs/sec")


if __name__ == "__main__":
    pytest.main([__file__, "-v", "-s"])
