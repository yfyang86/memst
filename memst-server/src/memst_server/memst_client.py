"""MemSt client wrapper for session and message management."""

import sys
from pathlib import Path
from typing import Optional, List, Dict, Any

# Add memst-py to path
memst_py_path = Path(__file__).parent.parent.parent / "memst-py" / "target" / "python"
if memst_py_path.exists():
    sys.path.insert(0, str(memst_py_path))

try:
    import memst
    MEMST_AVAILABLE = True
except ImportError:
    MEMST_AVAILABLE = False
    print("Warning: memst module not available. Install with: cd memst-py && maturin develop")


class MemStClient:
    """Client wrapper for MemSt session store."""

    def __init__(self, store_path: str = "./memst-store"):
        self.store_path = Path(store_path)
        self._store = None

        if MEMST_AVAILABLE:
            self._init_store()
        else:
            print(f"Warning: MemSt not available, store path: {self.store_path}")

    def _init_store(self):
        """Initialize the session store."""
        try:
            self._store = memst.SessionStore(str(self.store_path))
        except Exception as e:
            print(f"Warning: Failed to initialize MemSt store: {e}")
            self._store = None

    @property
    def is_available(self) -> bool:
        """Check if MemSt is available."""
        return MEMST_AVAILABLE and self._store is not None

    def create_session(self, name: str, model: str, user_id: Optional[str] = None) -> Optional[Dict[str, Any]]:
        """Create a new session."""
        if not self.is_available:
            return None

        try:
            session = self._store.create_session(name, model)
            return {
                "id": session.id,
                "name": session.name,
                "model": session.model,
                "user_id": user_id or "",
                "created_at": session.created_at,
                "updated_at": session.created_at,
                "message_count": session.message_count,
                "tags": [],
                "status": "active",
                "session_type": "chat",
            }
        except Exception as e:
            print(f"Error creating session: {e}")
            return None

    def list_sessions(self) -> List[Dict[str, Any]]:
        """List all sessions."""
        memst_sessions = []
        if self.is_available:
            try:
                memst_sessions = self._store.list_sessions()
            except Exception as e:
                print(f"Error listing sessions from MemSt: {e}")

        # If MemSt returned sessions, use them
        if memst_sessions:
            return [
                {
                    "id": s.get("id", ""),
                    "name": s.get("name", ""),
                    "model": s.get("model", ""),
                    "user_id": "",
                    "created_at": s.get("created_at", ""),
                    "updated_at": s.get("created_at", ""),
                    "message_count": s.get("message_count", 0),
                    "tags": [],
                    "status": "active",
                    "session_type": "chat",
                }
                for s in memst_sessions
            ]

        # Fallback: read sessions directly from filesystem
        return self._list_sessions_from_filesystem()

    def _list_sessions_from_filesystem(self) -> List[Dict[str, Any]]:
        """List sessions from filesystem when MemSt is unavailable."""
        sessions = []
        sessions_path = self.store_path / "sessions"

        if not sessions_path.exists():
            return []

        for session_dir in sessions_path.iterdir():
            if session_dir.is_dir():
                metadata_file = session_dir / "metadata.json"
                if metadata_file.exists():
                    try:
                        import json
                        with open(metadata_file) as f:
                            metadata = json.load(f)
                        sessions.append({
                            "id": session_dir.name,
                            "name": metadata.get("name", "Untitled"),
                            "model": metadata.get("model", ""),
                            "user_id": "",
                            "created_at": metadata.get("created_at", ""),
                            "updated_at": metadata.get("last_activity", ""),
                            "message_count": metadata.get("message_count", 0),
                            "tags": metadata.get("tags", []),
                            "status": "active",
                            "session_type": "chat",
                        })
                    except Exception as e:
                        print(f"Error reading session {session_dir.name}: {e}")

        # Sort by updated_at descending
        sessions.sort(key=lambda s: s.get("updated_at", ""), reverse=True)
        return sessions

    def get_session(self, session_id: str) -> Optional[Dict[str, Any]]:
        """Get session by ID."""
        if self.is_available:
            try:
                session = self._store.get_session(session_id)
                if session:
                    return {
                        "id": session_id,
                        "name": session.get("name", ""),
                        "model": session.get("model", ""),
                        "user_id": "",
                        "created_at": session.get("created_at", ""),
                        "updated_at": session.get("created_at", ""),
                        "message_count": session.get("message_count", 0),
                        "tags": [],
                        "status": "active",
                        "session_type": "chat",
                    }
            except Exception as e:
                print(f"Error getting session from MemSt: {e}")

        # Fallback: read from filesystem
        return self._get_session_from_filesystem(session_id)

    def _get_session_from_filesystem(self, session_id: str) -> Optional[Dict[str, Any]]:
        """Get session from filesystem."""
        session_dir = self.store_path / "sessions" / session_id
        metadata_file = session_dir / "metadata.json"

        if not metadata_file.exists():
            return None

        try:
            import json
            with open(metadata_file) as f:
                metadata = json.load(f)
            return {
                "id": session_id,
                "name": metadata.get("name", "Untitled"),
                "model": metadata.get("model", ""),
                "user_id": "",
                "created_at": metadata.get("created_at", ""),
                "updated_at": metadata.get("last_activity", ""),
                "message_count": metadata.get("message_count", 0),
                "tags": metadata.get("tags", []),
                "status": "active",
                "session_type": "chat",
            }
        except Exception as e:
            print(f"Error reading session {session_id} from filesystem: {e}")
            return None

    def delete_session(self, session_id: str) -> bool:
        """Delete session by ID."""
        if not self.is_available:
            return False

        try:
            self._store.delete_session(session_id)
            return True
        except Exception as e:
            print(f"Error deleting session: {e}")
            return False

    def add_message(self, session_id: str, role: str, content: str) -> Optional[Dict[str, Any]]:
        """Add message to session."""
        if not self.is_available:
            return None

        try:
            # Map role strings to MemSt Role enum
            role_mapping = {
                "user": memst.Role.User,
                "assistant": memst.Role.Assistant,
                "system": memst.Role.System,
                "function": memst.Role.Tool,
                "tool": memst.Role.Tool,
            }
            role_enum = role_mapping.get(role.lower(), memst.Role.User)
            self._store.add_message(session_id, role_enum, content)

            return {
                "id": "",
                "session_id": session_id,
                "role": role,
                "content": content,
                "timestamp": "",
                "attachments": [],
                "metadata": {},
            }
        except Exception as e:
            print(f"Error adding message: {e}")
            return None

    def get_messages(self, session_id: str) -> List[Dict[str, Any]]:
        """Get messages from session."""
        if not self.is_available:
            return []

        try:
            messages = self._store.get_session_messages(session_id)
            return [
                {
                    "id": m.get("id", ""),
                    "session_id": session_id,
                    "role": m.get("role", ""),
                    "content": m.get("content", ""),
                    "timestamp": m.get("timestamp", ""),
                    "attachments": [],
                    "metadata": {},
                }
                for m in messages
            ]
        except Exception as e:
            print(f"Error getting messages: {e}")
            return []

    def search(
        self,
        query: str,
        limit: int = 20,
        session_id: Optional[str] = None,
        search_type: str = "text",
    ) -> List[Dict[str, Any]]:
        """Search across sessions with text, semantic, or regex search."""
        if not self.is_available:
            return []

        try:
            # Map search_type to MemSt's search mode
            if search_type == "semantic":
                # MemSt's default search is semantic
                results = self._store.search(query, limit)
            elif search_type == "text":
                # Text search - use MemSt's text search if available, otherwise fallback
                try:
                    results = self._store.search(query, limit)
                except Exception:
                    # Fallback: get all messages and filter
                    results = self._text_search(query, limit)
            elif search_type == "regex":
                # Regex search - local implementation
                results = self._regex_search(query, limit)
            else:
                # Default to semantic
                results = self._store.search(query, limit)

            return [
                {
                    "id": r.get("id", ""),
                    "session_id": r.get("session_id", ""),
                    "doc_type": r.get("doc_type", ""),
                    "content": r.get("snippet", ""),
                    "score": r.get("score", 0.0),
                    "highlight": r.get("snippet", ""),
                }
                for r in results
            ]
        except Exception as e:
            print(f"Error searching: {e}")
            return []

    def _text_search(self, query: str, limit: int) -> List[Dict[str, Any]]:
        """Local text search fallback."""

        results = []
        try:
            sessions = self._store.list_sessions() if self._store else []
            for session in sessions:
                session_id = session.get("id", "")
                messages = self._store.get_session_messages(session_id) if self._store else []
                for msg in messages:
                    content = msg.get("content", "")
                    # Simple keyword matching (case insensitive)
                    if query.lower() in content.lower():
                        results.append({
                            "id": msg.get("id", ""),
                            "session_id": session_id,
                            "doc_type": "message",
                            "snippet": content[:200],
                            "score": 1.0,
                        })
        except Exception as e:
            print(f"Error in text search fallback: {e}")
        return results[:limit]

    def _regex_search(self, query: str, limit: int) -> List[Dict[str, Any]]:
        """Local regex search implementation."""
        import re

        results = []
        try:
            # Compile regex pattern
            pattern = re.compile(query, re.IGNORECASE)
            sessions = self._store.list_sessions() if self._store else []
            for session in sessions:
                session_id = session.get("id", "")
                messages = self._store.get_session_messages(session_id) if self._store else []
                for msg in messages:
                    content = msg.get("content", "")
                    if pattern.search(content):
                        results.append({
                            "id": msg.get("id", ""),
                            "session_id": session_id,
                            "doc_type": "message",
                            "snippet": content[:200],
                            "score": 1.0,
                        })
        except re.error as e:
            print(f"Invalid regex pattern: {e}")
        except Exception as e:
            print(f"Error in regex search: {e}")
        return results[:limit]

    def add_memory(
        self,
        session_id: str,
        tier: str,
        content: str,
        tags: Optional[List[str]] = None,
    ) -> bool:
        """Add memory to session."""
        if not self.is_available:
            return False

        try:
            tier_enum = getattr(memst.MemoryTier, tier.capitalize(), memst.MemoryTier.Working)
            self._store.add_memory(session_id, tier_enum, content, tags or [])
            return True
        except Exception as e:
            print(f"Error adding memory: {e}")
            return False

    def get_memories(self, session_id: str, tier: str) -> List[Dict[str, Any]]:
        """Get memories from session tier."""
        if not self.is_available:
            return []

        try:
            tier_enum = getattr(memst.MemoryTier, tier.capitalize(), memst.MemoryTier.Working)
            memories = self._store.get_session_memory(session_id, tier_enum)
            return [
                {
                    "id": m.get("id", ""),
                    "content": m.get("content", ""),
                    "source": m.get("source", ""),
                    "tags": m.get("tags", []),
                    "confidence": m.get("confidence", 0.8),
                    "importance": m.get("importance", 0.5),
                    "access_count": m.get("access_count", 0),
                }
                for m in memories
            ]
        except Exception as e:
            print(f"Error getting memories: {e}")
            return []


# Global client instance
_client: Optional[MemStClient] = None


def get_memst_client() -> MemStClient:
    """Get global MemSt client instance."""
    global _client
    if _client is None:
        from memst_server.config import get_config
        config = get_config()
        _client = MemStClient(config.server.store_path)
    return _client
