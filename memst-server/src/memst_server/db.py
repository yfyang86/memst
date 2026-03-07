"""DuckDB user and settings management for MemSt server."""

import json
import sqlite3
from pathlib import Path
from datetime import datetime
from typing import Optional, List, Dict, Any
import uuid

from memst_server.config import load_config, find_config_file


def get_default_settings_from_config() -> Dict[str, Any]:
    """Load default settings from config.toml if available."""
    config_path = find_config_file()
    if config_path and config_path.exists():
        config = load_config(config_path)
        return {
            "llm": {
                "type": config.llm.type,
                "api_url": config.llm.get_api_url(),
                "model": config.llm.model,
                "timeout": config.llm.timeout,
                "max_tokens": config.llm.max_tokens,
                "temperature": config.llm.temperature,
                # NOTE: api_key intentionally not exposed via API for security
            },
            "embedding": {
                "type": config.embedding.type,
                "api_url": config.embedding.get_api_url(),
                "model": config.embedding.model,
                "timeout": config.embedding.timeout,
                "expected_dimension": config.embedding.expected_dimension,
            },
            "server": {
                "host": config.server.host,
                "port": config.server.port,
                "store_path": config.server.store_path,
            },
            "kg_extraction": {
                "enabled": config.kg_extraction.enabled,
                "db_path": config.kg_extraction.db_path,
                "default_ontology": config.kg_extraction.default_ontology,
            },
        }

    # Fallback to hardcoded defaults
    return {
        "llm": {
            "type": "openai",
            "api_url": "http://localhost:8080/v1",
            "model": "gpt-4",
            "timeout": 60,
            "max_tokens": 8192,
            "temperature": 0.7,
            "api_key": "",
        },
        "embedding": {
            "type": "openai",
            "api_url": "http://localhost:8081/v1/embeddings",
            "model": "text-embedding-bge_m3",
            "timeout": 30,
            "expected_dimension": 1024,
        },
        "server": {
            "host": "127.0.0.1",
            "port": 8192,
            "store_path": "./memst-store",
        },
        "kg_extraction": {
            "enabled": True,
            "db_path": None,
            "default_ontology": None,
        },
    }


DEFAULT_SETTINGS = get_default_settings_from_config()


class User:
    """User model."""
    def __init__(
        self,
        id: str,
        name: str,
        avatar: Optional[str] = None,
        created_at: Optional[str] = None,
        settings: Optional[dict] = None,
    ):
        self.id = id
        self.name = name
        self.avatar = avatar
        self.created_at = created_at or datetime.utcnow().isoformat()
        self.settings = settings or {
            "theme": "dark",
            "default_model": "",
            "store_path": "./memst-store",
        }

    def to_dict(self) -> dict:
        return {
            "id": self.id,
            "name": self.name,
            "avatar": self.avatar,
            "created_at": self.created_at,
            "settings": self.settings,
        }

    @classmethod
    def from_dict(cls, data: dict) -> "User":
        return cls(
            id=data["id"],
            name=data["name"],
            avatar=data.get("avatar"),
            created_at=data.get("created_at"),
            settings=data.get("settings", {}),
        )


class UserDatabase:
    """DuckDB-based user database."""

    def __init__(self, db_path: str = "./data/memst-users.db"):
        self.db_path = Path(db_path)
        self._init_db()

    def _get_connection(self):
        """Get SQLite connection (DuckDB compatible)."""
        conn = sqlite3.connect(str(self.db_path))
        conn.row_factory = sqlite3.Row
        return conn

    def _init_db(self):
        """Initialize the database schema."""
        # Ensure parent directory exists
        self.db_path.parent.mkdir(parents=True, exist_ok=True)
        conn = self._get_connection()
        cursor = conn.cursor()

        cursor.execute("""
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                avatar TEXT,
                created_at TEXT NOT NULL,
                settings TEXT NOT NULL
            )
        """)

        cursor.execute("""
            CREATE TABLE IF NOT EXISTS settings (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                llm TEXT NOT NULL,
                embedding TEXT NOT NULL,
                server TEXT NOT NULL,
                kg_extraction TEXT,
                updated_at TEXT NOT NULL
            )
        """)

        # Sessions table for metadata (session content is managed by MemSt)
        cursor.execute("""
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                name TEXT NOT NULL,
                session_type TEXT NOT NULL DEFAULT 'chat',
                model TEXT NOT NULL,
                store_path TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                message_count INTEGER DEFAULT 0,
                tags TEXT,
                FOREIGN KEY (user_id) REFERENCES users(id)
            )
        """)

        # Insert default settings if not exists
        cursor.execute("SELECT COUNT(*) FROM settings")
        if cursor.fetchone()[0] == 0:
            defaults = DEFAULT_SETTINGS
            cursor.execute(
                "INSERT INTO settings (id, llm, embedding, server, kg_extraction, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
                (1, json.dumps(defaults["llm"]), json.dumps(defaults["embedding"]),
                 json.dumps(defaults["server"]), json.dumps(defaults.get("kg_extraction", {})),
                 datetime.utcnow().isoformat())
            )

        conn.commit()
        conn.close()

    def create_user(self, name: str, avatar: Optional[str] = None) -> User:
        """Create a new user."""
        conn = self._get_connection()
        cursor = conn.cursor()

        user = User(
            id=str(uuid.uuid4()),
            name=name,
            avatar=avatar,
            created_at=datetime.utcnow().isoformat(),
        )

        cursor.execute(
            "INSERT INTO users (id, name, avatar, created_at, settings) VALUES (?, ?, ?, ?, ?)",
            (user.id, user.name, user.avatar, user.created_at, json.dumps(user.settings)),
        )

        conn.commit()
        conn.close()
        return user

    def get_user(self, user_id: str) -> Optional[User]:
        """Get user by ID."""
        conn = self._get_connection()
        cursor = conn.cursor()

        cursor.execute("SELECT * FROM users WHERE id = ?", (user_id,))
        row = cursor.fetchone()

        conn.close()

        if row:
            return User.from_dict({
                "id": row["id"],
                "name": row["name"],
                "avatar": row["avatar"],
                "created_at": row["created_at"],
                "settings": json.loads(row["settings"]),
            })
        return None

    def list_users(self) -> List[User]:
        """List all users."""
        conn = self._get_connection()
        cursor = conn.cursor()

        cursor.execute("SELECT * FROM users ORDER BY created_at DESC")
        rows = cursor.fetchall()

        conn.close()

        return [
            User.from_dict({
                "id": row["id"],
                "name": row["name"],
                "avatar": row["avatar"],
                "created_at": row["created_at"],
                "settings": json.loads(row["settings"]),
            })
            for row in rows
        ]

    def update_user(self, user_id: str, name: Optional[str] = None, avatar: Optional[str] = None, settings: Optional[dict] = None) -> Optional[User]:
        """Update user."""
        conn = self._get_connection()
        cursor = conn.cursor()

        updates = []
        params = []

        if name is not None:
            updates.append("name = ?")
            params.append(name)
        if avatar is not None:
            updates.append("avatar = ?")
            params.append(avatar)
        if settings is not None:
            updates.append("settings = ?")
            params.append(json.dumps(settings))

        if not updates:
            return self.get_user(user_id)

        params.append(user_id)
        cursor.execute(f"UPDATE users SET {', '.join(updates)} WHERE id = ?", params)

        conn.commit()
        conn.close()

        return self.get_user(user_id)

    def delete_user(self, user_id: str) -> bool:
        """Delete user by ID."""
        conn = self._get_connection()
        cursor = conn.cursor()

        cursor.execute("DELETE FROM users WHERE id = ?", (user_id,))
        deleted = cursor.rowcount > 0

        conn.commit()
        conn.close()
        return deleted

    # =============================================================================
    # Settings Methods
    # =============================================================================

    def get_settings(self) -> Dict[str, Any]:
        """Get all settings."""
        conn = self._get_connection()
        cursor = conn.cursor()

        cursor.execute("SELECT llm, embedding, server, kg_extraction FROM settings WHERE id = 1")
        row = cursor.fetchone()

        conn.close()

        if row:
            result = {
                "llm": json.loads(row["llm"]),
                "embedding": json.loads(row["embedding"]),
                "server": json.loads(row["server"]),
            }
            # kg_extraction may be null in older databases
            if row["kg_extraction"]:
                result["kg_extraction"] = json.loads(row["kg_extraction"])
            else:
                result["kg_extraction"] = DEFAULT_SETTINGS.get("kg_extraction", {})
            return result
        return DEFAULT_SETTINGS.copy()

    def update_settings(
        self,
        llm: Optional[Dict[str, Any]] = None,
        embedding: Optional[Dict[str, Any]] = None,
        server: Optional[Dict[str, Any]] = None,
        kg_extraction: Optional[Dict[str, Any]] = None,
    ) -> Dict[str, Any]:
        """Update settings."""
        conn = self._get_connection()
        cursor = conn.cursor()

        current = self.get_settings()

        new_llm = llm if llm is not None else current["llm"]
        new_embedding = embedding if embedding is not None else current["embedding"]
        new_server = server if server is not None else current["server"]
        new_kg = kg_extraction if kg_extraction is not None else current.get("kg_extraction", {})

        cursor.execute(
            "UPDATE settings SET llm = ?, embedding = ?, server = ?, kg_extraction = ?, updated_at = ? WHERE id = 1",
            (
                json.dumps(new_llm),
                json.dumps(new_embedding),
                json.dumps(new_server),
                json.dumps(new_kg),
                datetime.utcnow().isoformat(),
            )
        )

        conn.commit()
        conn.close()

        return {
            "llm": new_llm,
            "embedding": new_embedding,
            "server": new_server,
            "kg_extraction": new_kg,
        }

    def reset_settings(self) -> Dict[str, Any]:
        """Reset settings to defaults (from config.toml if available)."""
        conn = self._get_connection()
        cursor = conn.cursor()

        defaults = DEFAULT_SETTINGS
        cursor.execute(
            "UPDATE settings SET llm = ?, embedding = ?, server = ?, kg_extraction = ?, updated_at = ? WHERE id = 1",
            (
                json.dumps(defaults["llm"]),
                json.dumps(defaults["embedding"]),
                json.dumps(defaults["server"]),
                json.dumps(defaults.get("kg_extraction", {})),
                datetime.utcnow().isoformat(),
            )
        )

        conn.commit()
        conn.close()

        return {
            "llm": defaults["llm"].copy(),
            "embedding": defaults["embedding"].copy(),
            "server": defaults["server"].copy(),
            "kg_extraction": defaults.get("kg_extraction", {}).copy(),
        }

    # =============================================================================
    # Session Metadata Methods (session content is managed by MemSt)
    # =============================================================================

    def create_session_metadata(
        self,
        session_id: str,
        user_id: str,
        name: str,
        session_type: str,
        model: str,
        store_path: str,
        message_count: int = 0,
        tags: Optional[List[str]] = None,
    ) -> bool:
        """Create session metadata in DuckDB (session content managed by MemSt)."""
        conn = self._get_connection()
        cursor = conn.cursor()

        now = datetime.utcnow().isoformat()
        try:
            cursor.execute(
                """INSERT INTO sessions
                   (id, user_id, name, session_type, model, store_path, created_at, updated_at, message_count, tags)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)""",
                (session_id, user_id, name, session_type, model, store_path, now, now, message_count, json.dumps(tags or [])),
            )
            conn.commit()
            return True
        except Exception as e:
            print(f"Error creating session metadata: {e}")
            return False
        finally:
            conn.close()

    def get_session_metadata(self, session_id: str) -> Optional[Dict[str, Any]]:
        """Get session metadata by ID."""
        conn = self._get_connection()
        cursor = conn.cursor()

        cursor.execute("SELECT * FROM sessions WHERE id = ?", (session_id,))
        row = cursor.fetchone()
        conn.close()

        if row:
            return {
                "id": row["id"],
                "user_id": row["user_id"],
                "name": row["name"],
                "session_type": row["session_type"],
                "model": row["model"],
                "store_path": row["store_path"],
                "created_at": row["created_at"],
                "updated_at": row["updated_at"],
                "message_count": row["message_count"],
                "tags": json.loads(row["tags"]),
            }
        return None

    def list_sessions_metadata(self, user_id: Optional[str] = None, session_type: Optional[str] = None) -> List[Dict[str, Any]]:
        """List session metadata with optional filtering."""
        conn = self._get_connection()
        cursor = conn.cursor()

        query = "SELECT * FROM sessions"
        params = []
        conditions = []

        if user_id:
            conditions.append("user_id = ?")
            params.append(user_id)
        if session_type:
            conditions.append("session_type = ?")
            params.append(session_type)

        if conditions:
            query += " WHERE " + " AND ".join(conditions)

        query += " ORDER BY updated_at DESC"

        cursor.execute(query, params)
        rows = cursor.fetchall()
        conn.close()

        return [
            {
                "id": row["id"],
                "user_id": row["user_id"],
                "name": row["name"],
                "session_type": row["session_type"],
                "model": row["model"],
                "store_path": row["store_path"],
                "created_at": row["created_at"],
                "updated_at": row["updated_at"],
                "message_count": row["message_count"],
                "tags": json.loads(row["tags"]),
            }
            for row in rows
        ]

    def update_session_metadata(
        self,
        session_id: str,
        name: Optional[str] = None,
        session_type: Optional[str] = None,
        model: Optional[str] = None,
        message_count: Optional[int] = None,
        tags: Optional[List[str]] = None,
    ) -> bool:
        """Update session metadata."""
        conn = self._get_connection()
        cursor = conn.cursor()

        updates = []
        params = []

        if name is not None:
            updates.append("name = ?")
            params.append(name)
        if session_type is not None:
            updates.append("session_type = ?")
            params.append(session_type)
        if model is not None:
            updates.append("model = ?")
            params.append(model)
        if message_count is not None:
            updates.append("message_count = ?")
            params.append(message_count)
        if tags is not None:
            updates.append("tags = ?")
            params.append(json.dumps(tags))

        if not updates:
            return False

        updates.append("updated_at = ?")
        params.append(datetime.utcnow().isoformat())
        params.append(session_id)

        cursor.execute(f"UPDATE sessions SET {', '.join(updates)} WHERE id = ?", params)
        conn.commit()
        conn.close()
        return True

    def delete_session_metadata(self, session_id: str) -> bool:
        """Delete session metadata."""
        conn = self._get_connection()
        cursor = conn.cursor()

        cursor.execute("DELETE FROM sessions WHERE id = ?", (session_id,))
        deleted = cursor.rowcount > 0
        conn.commit()
        conn.close()
        return deleted

    def sync_sessions_from_memst(self, memst_store_path: str, user_id: str = "default") -> int:
        """Sync session metadata from MemSt filesystem store to DuckDB."""
        conn = self._get_connection()
        cursor = conn.cursor()

        sync_count = 0
        sessions_path = Path(memst_store_path) / "sessions"

        if not sessions_path.exists():
            return 0

        for session_dir in sessions_path.iterdir():
            if session_dir.is_dir():
                session_id = session_dir.name
                metadata_file = session_dir / "metadata.json"

                if metadata_file.exists():
                    try:
                        with open(metadata_file) as f:
                            metadata = json.load(f)

                        # Check if session already exists
                        cursor.execute("SELECT id FROM sessions WHERE id = ?", (session_id,))
                        if cursor.fetchone() is None:
                            now = datetime.utcnow().isoformat()
                            cursor.execute(
                                """INSERT INTO sessions
                                   (id, user_id, name, session_type, model, store_path, created_at, updated_at, message_count, tags)
                                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)""",
                                (
                                    session_id,
                                    user_id,
                                    metadata.get("name", "Untitled"),
                                    "chat",  # default type
                                    metadata.get("model", ""),
                                    memst_store_path,
                                    metadata.get("created_at", now),
                                    metadata.get("last_activity", now),
                                    metadata.get("message_count", 0),
                                    json.dumps(metadata.get("tags", [])),
                                ),
                            )
                            sync_count += 1
                    except Exception as e:
                        print(f"Error syncing session {session_id}: {e}")

        conn.commit()
        conn.close()
        return sync_count
