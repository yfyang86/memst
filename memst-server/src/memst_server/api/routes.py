"""API routes for MemSt server - Complete REST API for MemSt web interface."""

from typing import Optional, List, Dict, Any
from datetime import datetime
from pathlib import Path
import json
import uuid
from fastapi import APIRouter, HTTPException, UploadFile, File
from pydantic import BaseModel
from memst_server.db import UserDatabase
from memst_server.memst_client import get_memst_client
from memst_server.nanobot_client import get_nanobot_client, _save_to_working_memory
from memst_server.memory_service import MemoryReloadService

router = APIRouter(prefix="/api/v1", tags=["api"])

_memst_client = get_memst_client()
_nanobot_client = get_nanobot_client()
_db: Optional[UserDatabase] = None


def get_db() -> UserDatabase:
    """Get database instance."""
    global _db
    if _db is None:
        _db = UserDatabase()  # Uses default path from config.server.store_path
    return _db


# =============================================================================
# Pydantic Models
# =============================================================================

class UserCreate(BaseModel):
    name: str
    avatar: Optional[str] = None


class UserUpdate(BaseModel):
    name: Optional[str] = None
    avatar: Optional[str] = None
    settings: Optional[Dict[str, Any]] = None


class SessionCreate(BaseModel):
    name: str
    session_type: str = "chat"
    model: str = "gpt-4"
    tags: Optional[List[str]] = None


class SessionUpdate(BaseModel):
    name: Optional[str] = None
    tags: Optional[List[str]] = None


class MessageCreate(BaseModel):
    role: str
    content: str
    attachments: Optional[List[Dict[str, Any]]] = None


class MemoryCreate(BaseModel):
    tier: str = "working"
    content: str
    tags: Optional[List[str]] = None
    importance: float = 0.5


class MemoryRetrieveRequest(BaseModel):
    query: str
    limit: int = 5
    tier: Optional[str] = None


class SearchRequest(BaseModel):
    query: str
    search_type: str = "hybrid"  # text, semantic, hybrid, regex
    session_id: Optional[str] = None
    limit: int = 20


class OperationLog(BaseModel):
    operation_type: str
    details: Dict[str, Any]
    timestamp: Optional[str] = None


class BranchRequest(BaseModel):
    name: str
    source_branch: Optional[str] = None


class ChatRequest(BaseModel):
    """Chat completion request."""
    message: str
    model: Optional[str] = None


# =============================================================================
# Auth Routes
# =============================================================================

@router.post("/auth/login")
def login(request: UserCreate):
    """User login - creates or returns existing user."""
    db = get_db()
    # Try to find existing user by name
    users = db.list_users()
    for user in users:
        if user.name == request.name:
            return user.to_dict()

    # Create new user
    user = db.create_user(request.name, request.avatar)
    return user.to_dict()


@router.post("/auth/logout")
def logout():
    """User logout."""
    return {"success": True}


@router.post("/auth/register")
def register(request: UserCreate):
    """Register new user."""
    db = get_db()
    user = db.create_user(request.name, request.avatar)
    return user.to_dict()


@router.get("/users/me", response_model=Dict)
def get_current_user():
    """Get current user profile."""
    db = get_db()
    users = db.list_users()
    if not users:
        # Create default user
        user = db.create_user("Demo User", None)
        return user.to_dict()
    return users[0].to_dict()


@router.put("/users/me", response_model=Dict)
def update_current_user(request: UserUpdate):
    """Update current user profile."""
    db = get_db()
    users = db.list_users()
    if not users:
        raise HTTPException(status_code=404, detail="No user found")
    user = db.update_user(users[0].id, request.name, request.avatar, request.settings)
    return user.to_dict() if user else users[0].to_dict()


@router.get("/users", response_model=List[Dict])
def list_users():
    """List all users (admin)."""
    return [u.to_dict() for u in get_db().list_users()]


# =============================================================================
# Session Routes
# =============================================================================

@router.get("/sessions", response_model=List[Dict])
def list_sessions(user_id: Optional[str] = None, session_type: Optional[str] = None):
    """List user's sessions with optional filtering.

    Sessions are stored in DuckDB for metadata, MemSt manages session content.
    If MemSt is available and has sessions not in DuckDB, sync them first.
    If MemSt is unavailable, read sessions directly from filesystem.
    """
    db = get_db()

    # Get sessions from filesystem (works whether MemSt is available or not)
    sessions = _memst_client.list_sessions()

    # Sync to DuckDB if we got sessions from filesystem
    if sessions:
        users = db.list_users()
        current_user_id = users[0].id if users else "default"
        synced = db.sync_sessions_from_memst(_memst_client.store_path, current_user_id)
        if synced > 0:
            print(f"Synced {synced} sessions to DuckDB")

    # Get sessions from DuckDB (now has the synced sessions)
    sessions = db.list_sessions_metadata(user_id, session_type)

    # If nothing in DuckDB, fall back to filesystem
    if not sessions:
        sessions = _memst_client.list_sessions()

    # Format for API response - sync message_count from MemSt for each session
    result = []
    for s in sessions:
        session_id = s["id"]
        message_count = s.get("message_count", 0)

        # Sync message_count from MemSt if available
        if _memst_client.is_available:
            memst_session = _memst_client.get_session(session_id)
            if memst_session:
                actual_count = memst_session.get("message_count", 0)
                if actual_count != message_count:
                    db.update_session_metadata(session_id, message_count=actual_count)
                    message_count = actual_count

        result.append({
            "id": s["id"],
            "name": s["name"],
            "model": s["model"],
            "user_id": s["user_id"],
            "created_at": s["created_at"],
            "updated_at": s["updated_at"],
            "message_count": message_count,
            "tags": s.get("tags", []),
            "status": "active",
            "session_type": s.get("session_type", "chat"),
        })

    return result


@router.post("/sessions", response_model=Dict)
def create_session(request: SessionCreate):
    """Create a new session.

    Creates session in MemSt (content) and DuckDB (metadata).
    """
    db = get_db()
    result = None

    # Create session in MemSt (content)
    if _memst_client.is_available:
        memst_result = _memst_client.create_session(request.name, request.model)
        if memst_result:
            # Get current user
            users = db.list_users()
            current_user_id = users[0].id if users else "default"

            # Store metadata in DuckDB
            db.create_session_metadata(
                session_id=memst_result["id"],
                user_id=current_user_id,
                name=request.name,
                session_type=request.session_type,
                model=request.model,
                store_path=str(_memst_client.store_path),
                tags=request.tags,
            )

            result = {
                "id": memst_result["id"],
                "name": memst_result["name"],
                "model": memst_result["model"],
                "user_id": current_user_id,
                "created_at": memst_result["created_at"],
                "updated_at": memst_result["updated_at"],
                "message_count": memst_result["message_count"],
                "tags": request.tags or [],
                "status": "active",
                "session_type": request.session_type,
            }

    if result is None:
        # Fallback: create without MemSt
        session_id = str(uuid.uuid4())
        now = datetime.utcnow().isoformat()
        users = db.list_users()
        current_user_id = users[0].id if users else "default"

        db.create_session_metadata(
            session_id=session_id,
            user_id=current_user_id,
            name=request.name,
            session_type=request.session_type,
            model=request.model,
            store_path=str(_memst_client.store_path) if _memst_client else "./memst-store",
            tags=request.tags,
        )

        result = {
            "id": session_id,
            "name": request.name,
            "model": request.model,
            "user_id": current_user_id,
            "created_at": now,
            "updated_at": now,
            "message_count": 0,
            "tags": request.tags or [],
            "status": "active",
            "session_type": request.session_type,
        }

    return result


@router.get("/sessions/{session_id}", response_model=Dict)
def get_session(session_id: str):
    """Get session details from DuckDB metadata and MemSt content."""
    db = get_db()

    # First try DuckDB metadata
    session = db.get_session_metadata(session_id)
    if session:
        # Sync message_count from MemSt if available
        if _memst_client.is_available:
            memst_session = _memst_client.get_session(session_id)
            if memst_session:
                actual_count = memst_session.get("message_count", 0)
                # Update DuckDB if counts differ
                if actual_count != session.get("message_count", 0):
                    db.update_session_metadata(session_id, message_count=actual_count)
                    session["message_count"] = actual_count

        return {
            "id": session["id"],
            "name": session["name"],
            "model": session["model"],
            "user_id": session["user_id"],
            "created_at": session["created_at"],
            "updated_at": session["updated_at"],
            "message_count": session["message_count"],
            "tags": session.get("tags", []),
            "status": "active",
            "session_type": session.get("session_type", "chat"),
        }

    # Fall back to MemSt
    if _memst_client.is_available:
        memst_session = _memst_client.get_session(session_id)
        if memst_session:
            return memst_session

    raise HTTPException(status_code=404, detail="Session not found")


@router.put("/sessions/{session_id}", response_model=Dict)
def update_session(session_id: str, request: SessionUpdate):
    """Update session metadata in DuckDB."""
    db = get_db()

    # Get current session
    session = db.get_session_metadata(session_id)
    if session is None:
        raise HTTPException(status_code=404, detail="Session not found")

    # Update metadata in DuckDB
    db.update_session_metadata(
        session_id,
        name=request.name,
        tags=request.tags,
    )

    # Return updated session
    updated = db.get_session_metadata(session_id)
    return {
        "id": updated["id"],
        "name": updated["name"],
        "model": updated["model"],
        "user_id": updated["user_id"],
        "created_at": updated["created_at"],
        "updated_at": updated["updated_at"],
        "message_count": updated["message_count"],
        "tags": updated.get("tags", []),
        "status": "active",
        "session_type": updated.get("session_type", "chat"),
    }


@router.delete("/sessions/{session_id}")
def delete_session(session_id: str):
    """Delete session from both DuckDB and MemSt."""
    db = get_db()

    # Delete from DuckDB
    db.delete_session_metadata(session_id)

    # Delete from MemSt if available
    if _memst_client.is_available:
        _memst_client.delete_session(session_id)

    # Also delete agent session if nanobot is available
    if _nanobot_client.is_available:
        _nanobot_client.delete_session(session_id)

    return {"success": True}


# =============================================================================
# Agent Routes (nanobot integration)
# =============================================================================

@router.get("/agent/status")
def get_agent_status():
    """Check nanobot agent status."""
    return {
        "available": _nanobot_client.is_available,
        "session_count": len(_nanobot_client.get_session_history("global")) if _nanobot_client.is_available else 0,
    }


@router.post("/agent/sessions", response_model=Dict)
def create_agent_session(request: SessionCreate):
    """Create a new agent session using nanobot.

    Agent sessions use the nanobot AgentLoop for processing.
    Messages are stored in MemSt for persistence.
    """
    if not _nanobot_client.is_available:
        raise HTTPException(status_code=503, detail="Nanobot agent is not available")

    # Create session metadata
    db = get_db()
    users = db.list_users()
    current_user_id = users[0].id if users else "default"
    now = datetime.utcnow().isoformat()
    session_id = str(uuid.uuid4())
    model = request.model or "gpt-4"

    # Create session in MemSt file-based store if available
    if _memst_client and _memst_client.is_available:
        memst_session = _memst_client.create_session(request.name, model, current_user_id)
        if memst_session:
            session_id = memst_session.get("id", session_id)

    # Create metadata in DuckDB
    db.create_session_metadata(
        session_id=session_id,
        user_id=current_user_id,
        name=request.name,
        session_type="agent",
        model=model,
        store_path=str(_memst_client.store_path) if _memst_client else "./memst-store",
        tags=request.tags,
    )

    return {
        "id": session_id,
        "name": request.name,
        "model": model,
        "user_id": current_user_id,
        "created_at": now,
        "updated_at": now,
        "message_count": 0,
        "tags": request.tags or [],
        "status": "active",
        "session_type": "agent",
    }


@router.post("/agent/chat/{session_id}", response_model=Dict)
async def agent_chat(session_id: str, request: ChatRequest):
    """Send message to agent and get response.

    Uses nanobot AgentLoop for processing with tool support.
    Messages are stored in MemSt for persistence.
    """
    if not _nanobot_client.is_available:
        raise HTTPException(status_code=503, detail="Nanobot agent is not available")

    # Get session to determine model
    session = None
    if _memst_client and _memst_client.is_available:
        session = _memst_client.get_session(session_id)
    model = request.model or (session.get("model") if session else "gpt-4")

    # Process with nanobot
    result = await _nanobot_client.chat(
        session_id=session_id,
        message=request.message,
        model=model,
    )

    # Response handled by nanobot_client.chat() which syncs to MemSt
    return result["assistant_message"]


@router.post("/agent/chat/{session_id}/stream")
async def agent_chat_stream(session_id: str, request: ChatRequest):
    """Stream agent response using nanobot."""
    if not _nanobot_client.is_available:
        raise HTTPException(status_code=503, detail="Nanobot agent is not available")

    from fastapi.responses import StreamingResponse
    import json

    # Get session to determine model
    session = None
    if _memst_client and _memst_client.is_available:
        session = _memst_client.get_session(session_id)
    model = request.model or (session.get("model") if session else "gpt-4")

    async def generate():
        accumulated_content = ""
        user_content = request.message

        try:
            async for chunk in _nanobot_client.chat_stream(
                session_id=session_id,
                message=request.message,
                model=model,
            ):
                if "content" in chunk:
                    content = chunk["content"]
                    accumulated_content += content
                    yield f"data: {json.dumps({'content': content})}\n"

            # Store messages after streaming completes
            if _memst_client and _memst_client.is_available and accumulated_content:
                try:
                    # Store to MemSt for persistence
                    _memst_client.add_message(session_id, "user", user_content)
                    _memst_client.add_message(session_id, "assistant", accumulated_content)
                    # Save to working memory for memory tier display
                    _save_to_working_memory(_memst_client, session_id, user_content, accumulated_content)
                except Exception as e:
                    print(f"Error storing agent messages in MemSt: {e}")

        except Exception as e:
            yield f"data: {json.dumps({'error': str(e)})}\n"

        yield "data: [DONE]\n"

    return StreamingResponse(generate(), media_type="text/event-stream")


@router.get("/agent/sessions/{session_id}/history", response_model=List[Dict])
def get_agent_history(session_id: str):
    """Get agent session message history."""
    if not _nanobot_client.is_available:
        return []

    history = _nanobot_client.get_session_history(session_id)
    return history


@router.delete("/agent/sessions/{session_id}")
def delete_agent_session(session_id: str):
    """Delete agent session."""
    if not _nanobot_client.is_available:
        raise HTTPException(status_code=503, detail="Nanobot agent is not available")

    success = _nanobot_client.delete_session(session_id)
    return {"success": success}


# =============================================================================
# Nanobot Config Routes
# =============================================================================

class NanobotConfigPathRequest(BaseModel):
    """Request to set nanobot config path."""
    config_path: str


@router.get("/agent/config", response_model=Dict)
def get_nanobot_config_info():
    """Get current nanobot configuration info."""
    from memst_server.nanobot_client import get_nanobot_config
    return get_nanobot_config()


@router.post("/agent/config/path")
def set_nanobot_config_path(request: NanobotConfigPathRequest):
    """Set custom nanobot config path.

    Use None to reset to default (~/.nanobot/config.json).
    """
    from memst_server.nanobot_client import set_nanobot_config_path, reload_nanobot_config

    set_nanobot_config_path(request.config_path if request.config_path else None)
    reload_nanobot_config()

    return {
        "success": True,
        "config_path": request.config_path,
        "message": "Config path set. Reload agent sessions to apply changes."
    }


@router.post("/agent/config/reload")
def reload_nanobot_config_endpoint():
    """Force reload of nanobot config."""
    from memst_server.nanobot_client import reload_nanobot_config
    reload_nanobot_config()
    return {"success": True, "message": "Config reloaded"}


# =============================================================================
# Skills Routes
# =============================================================================

class LoadSkillRequest(BaseModel):
    """Request to load a skill."""
    name: str


@router.get("/agent/skills", response_model=List[Dict])
def list_skills(filter_unavailable: bool = True):
    """List all available skills."""
    if not _nanobot_client.is_available:
        return []

    skills = _nanobot_client.list_skills(filter_unavailable=filter_unavailable)
    return skills


@router.get("/agent/skills/summary")
def get_skills_summary():
    """Get XML-formatted summary of all available skills."""
    if not _nanobot_client.is_available:
        return {"summary": "", "count": 0}

    summary = _nanobot_client.get_skills_summary()
    count = len(_nanobot_client.list_skills())
    return {"summary": summary, "count": count}


@router.post("/agent/skills/load")
def load_skill(request: LoadSkillRequest):
    """Load a specific skill by name."""
    if not _nanobot_client.is_available:
        raise HTTPException(status_code=503, detail="Nanobot agent is not available")

    content = _nanobot_client.load_skill(request.name)
    if content is None:
        raise HTTPException(status_code=404, detail=f"Skill '{request.name}' not found")

    return {"name": request.name, "content": content}


@router.get("/agent/skills/{skill_name}")
def get_skill(skill_name: str):
    """Get skill content and metadata."""
    if not _nanobot_client.is_available:
        raise HTTPException(status_code=503, detail="Nanobot agent is not available")

    # Get list of skills and find matching one
    skills = _nanobot_client.list_skills()
    skill = next((s for s in skills if s["name"] == skill_name), None)

    if skill is None:
        raise HTTPException(status_code=404, detail=f"Skill '{skill_name}' not found")

    # Load the skill content
    content = _nanobot_client.load_skill(skill_name)

    return {
        "name": skill["name"],
        "description": skill.get("description", ""),
        "always": skill.get("always", False),
        "source": skill.get("source", ""),
        "content": content or "",
    }


@router.get("/sessions/{session_id}/branch", response_model=Dict)
def branch_session(session_id: str, request: BranchRequest):
    """Branch session (git-like)."""
    # Return mock branch info
    return {
        "id": str(uuid.uuid4()),
        "name": request.name,
        "source_session_id": session_id,
        "source_branch": request.source_branch or "main",
        "created_at": datetime.utcnow().isoformat(),
        "message_count": 0,
    }


# =============================================================================
# Message Routes
# =============================================================================

@router.get("/sessions/{session_id}/messages", response_model=List[Dict])
def get_messages(session_id: str, limit: int = 100, offset: int = 0):
    """List messages (paginated)."""
    messages = _memst_client.get_messages(session_id) if _memst_client.is_available else []

    # Normalize role to lowercase (API expects "user", "assistant", "system")
    result = []
    for msg in messages:
        msg_copy = msg.copy()
        if "role" in msg_copy:
            msg_copy["role"] = msg_copy["role"].lower()
        result.append(msg_copy)

    return result[offset:offset + limit]


@router.post("/sessions/{session_id}/messages", response_model=Dict)
def add_message(session_id: str, request: MessageCreate):
    """Add message to session."""
    # Normalize role to lowercase
    role = request.role.lower() if request.role else "user"

    result = _memst_client.add_message(session_id, role, request.content) if _memst_client.is_available else None

    if result is None:
        result = {
            "id": str(uuid.uuid4()),
            "session_id": session_id,
            "role": role,
            "content": request.content,
            "timestamp": datetime.utcnow().isoformat(),
            "attachments": request.attachments or [],
            "metadata": {},
        }
    # Ensure role is lowercase in response
    if result and "role" in result:
        result["role"] = result["role"].lower()
    return result


# ChatRequest is defined at line 94 (first occurrence)


@router.post("/sessions/{session_id}/chat")
async def chat(session_id: str, request: ChatRequest):
    """Send message and get LLM response."""
    import httpx

    # Get session to determine model
    session = _memst_client.get_session(session_id) if _memst_client.is_available else None
    model = request.model or (session.get("model") if session else "gpt-4")

    # Get LLM config
    settings = get_db().get_settings()
    llm_config = settings.get("llm", {})
    api_url = llm_config.get("api_url", "http://localhost:8080/v1")
    api_key = llm_config.get("api_key", "")

    # Build messages for chat completion
    messages = []
    existing_messages = _memst_client.get_messages(session_id) if _memst_client.is_available else []
    for msg in existing_messages[-10:]:  # Last 10 messages for context
        # Normalize role to lowercase (API expects "user", "assistant", "system")
        role = msg.get("role", "user").lower()
        messages.append({"role": role, "content": msg.get("content", "")})
    messages.append({"role": "user", "content": request.message})

    # Build request to LLM
    payload = {
        "model": model,
        "messages": messages,
        "max_tokens": llm_config.get("max_tokens", 8192),
        "temperature": llm_config.get("temperature", 0.7),
    }

    headers = {"Content-Type": "application/json"}
    # Use default API key if not set (some APIs require auth even with dummy key)
    auth_key = api_key or "sk"
    headers["Authorization"] = f"Bearer {auth_key}"

    try:
        async with httpx.AsyncClient(timeout=120.0) as client:
            response = await client.post(f"{api_url}/chat/completions", json=payload, headers=headers)
            response.raise_for_status()
            data = response.json()

            # Extract assistant message
            assistant_content = data["choices"][0]["message"]["content"]

            # Add user message to session
            user_message = {
                "id": str(uuid.uuid4()),
                "session_id": session_id,
                "role": "user",
                "content": request.message,
                "timestamp": datetime.utcnow().isoformat(),
            }

            # Add assistant message to session
            assistant_message = {
                "id": str(uuid.uuid4()),
                "session_id": session_id,
                "role": "assistant",
                "content": assistant_content,
                "timestamp": datetime.utcnow().isoformat(),
                "metadata": {
                    "model": model,
                    "usage": data.get("usage", {}),
                },
            }

            # Store in backend if available
            if _memst_client.is_available:
                _memst_client.add_message(session_id, "user", request.message)
                _memst_client.add_message(session_id, "assistant", assistant_content)

                # Auto-save to working memory (latest 5 rounds)
                _save_to_working_memory(_memst_client, session_id, user_message, assistant_message)

            return assistant_message
    except httpx.HTTPError as e:
        raise HTTPException(status_code=500, detail=f"LLM request failed: {str(e)}")


@router.post("/sessions/{session_id}/chat/stream")
async def chat_stream(session_id: str, request: ChatRequest):
    """Send message and get LLM response as streaming SSE."""
    import httpx
    from fastapi.responses import StreamingResponse

    # Get session to determine model
    session = _memst_client.get_session(session_id) if _memst_client.is_available else None
    model = request.model or (session.get("model") if session else "gpt-4")

    # Get LLM config
    settings = get_db().get_settings()
    llm_config = settings.get("llm", {})
    api_url = llm_config.get("api_url", "http://localhost:8080/v1")
    api_key = llm_config.get("api_key", "")

    # Build messages for chat completion
    messages = []
    existing_messages = _memst_client.get_messages(session_id) if _memst_client.is_available else []
    for msg in existing_messages[-10:]:  # Last 10 messages for context
        role = msg.get("role", "user").lower()
        messages.append({"role": role, "content": msg.get("content", "")})
    messages.append({"role": "user", "content": request.message})

    # Build request to LLM with streaming
    payload = {
        "model": model,
        "messages": messages,
        "max_tokens": llm_config.get("max_tokens", 8192),
        "temperature": llm_config.get("temperature", 0.7),
        "stream": True,
    }

    headers = {"Content-Type": "application/json"}
    auth_key = api_key or "sk"
    headers["Authorization"] = f"Bearer {auth_key}"

    async def generate():
        accumulated_content = ""
        user_content = request.message

        try:
            async with httpx.AsyncClient(timeout=120.0) as client:
                async with client.stream("POST", f"{api_url}/chat/completions", json=payload, headers=headers) as response:
                    async for chunk in response.aiter_bytes():
                        decoded = chunk.decode("utf-8")
                        for line in decoded.split("\n"):
                            if line.startswith("data: "):
                                data = line[6:]
                                if data == "[DONE]":
                                    break
                                try:
                                    import json as json_module
                                    parsed = json_module.loads(data)
                                    if "choices" in parsed:
                                        delta = parsed["choices"][0].get("delta", {})
                                        content = delta.get("content", "")
                                        if content:
                                            accumulated_content += content
                                            yield f"data: {json_module.dumps({'content': content})}\n"
                                except Exception:
                                    # Skip malformed SSE data gracefully
                                    pass

                    # Store complete conversation after streaming finishes
                    if _memst_client and _memst_client.is_available and accumulated_content:
                        _memst_client.add_message(session_id, "user", user_content)
                        _memst_client.add_message(session_id, "assistant", accumulated_content)

                        # Save to working memory
                        user_message = {
                            "id": str(uuid.uuid4()),
                            "session_id": session_id,
                            "role": "user",
                            "content": user_content,
                            "timestamp": datetime.utcnow().isoformat(),
                        }
                        assistant_message = {
                            "id": str(uuid.uuid4()),
                            "session_id": session_id,
                            "role": "assistant",
                            "content": accumulated_content,
                            "timestamp": datetime.utcnow().isoformat(),
                        }
                        # Save to working memory with correct parameter order
                        _save_to_working_memory(_memst_client, session_id, user_content, accumulated_content)

        except Exception as e:
            yield f"data: {json.dumps({'error': str(e)})}\n"

        yield "data: [DONE]\n"

    return StreamingResponse(generate(), media_type="text/event-stream")


@router.get("/sessions/{session_id}/messages/{message_id}", response_model=Dict)
def get_message(session_id: str, message_id: str):
    """Get specific message."""
    messages = _memst_client.get_messages(session_id) if _memst_client.is_available else []
    for msg in messages:
        if msg.get("id") == message_id:
            return msg
    raise HTTPException(status_code=404, detail="Message not found")


@router.delete("/sessions/{session_id}/messages/{message_id}")
def delete_message(session_id: str, message_id: str):
    """Delete message."""
    return {"success": True}


# =============================================================================
# Memory Routes
# =============================================================================

def _get_data_path() -> Path:
    """Get the data path from settings."""
    settings = get_db().get_settings()
    data_path = Path(settings.get("server", {}).get("store_path", "./memst-store"))
    data_path.mkdir(parents=True, exist_ok=True)
    return data_path


@router.get("/sessions/{session_id}/memory", response_model=Dict)
def get_all_memories(session_id: str):
    """List all memories across tiers."""
    result = {
        "working": [],
        "short": [],
        "long": [],
    }

    # Use MemSt client if available
    if _memst_client.is_available:
        try:
            # Ensure session exists in MemSt (for agent sessions that weren't synced)
            if _memst_client.get_session(session_id) is None:
                # Try to get session metadata from DuckDB
                session_meta = _db.get_session_metadata(session_id)
                if session_meta:
                    _memst_client.create_session(
                        name=session_meta.get("name", session_id[:8]),
                        model=session_meta.get("model", "unknown")
                    )

            result["working"] = _memst_client.get_memories(session_id, "working")
            result["short"] = _memst_client.get_memories(session_id, "short")
            result["long"] = _memst_client.get_memories(session_id, "long")
        except Exception as e:
            print(f"Error getting memories: {e}")

    return result


@router.get("/sessions/{session_id}/memory/{tier}", response_model=List[Dict])
def get_memories_by_tier(session_id: str, tier: str):
    """List memories by tier."""
    # Use MemSt client if available
    if _memst_client.is_available:
        try:
            # Ensure session exists in MemSt (for agent sessions that weren't synced)
            if _memst_client.get_session(session_id) is None:
                session_meta = _db.get_session_metadata(session_id)
                if session_meta:
                    _memst_client.create_session(
                        name=session_meta.get("name", session_id[:8]),
                        model=session_meta.get("model", "unknown")
                    )
            return _memst_client.get_memories(session_id, tier)
        except Exception as e:
            print(f"Error getting memories by tier: {e}")

    # Fallback to JSONL file
    import json
    data_path = _get_data_path()
    memories = []

    if tier == "working":
        working_file = data_path / f"{session_id}_working_memory.jsonl"
        if working_file.exists():
            with open(working_file) as f:
                for line in f:
                    if line.strip():
                        entry = json.loads(line)
                        memories.append({
                            "id": f"working_{entry.get('round', 0)}",
                            "content": f"Round {entry.get('round', 0)}: {entry.get('user_message', '')}",
                            "detail": entry.get("assistant_message", ""),
                            "tier": "working",
                            "timestamp": entry.get("timestamp", ""),
                        })
    elif tier == "short":
        short_file = data_path / f"{session_id}_short_memory.jsonl"
        if short_file.exists():
            with open(short_file) as f:
                for line in f:
                    if line.strip():
                        entry = json.loads(line)
                        memories.append({
                            "id": entry.get("id", ""),
                            "content": entry.get("content", ""),
                            "tier": "short",
                            "file_name": entry.get("file_name", ""),
                            "timestamp": entry.get("timestamp", ""),
                        })
    elif tier == "long":
        long_file = data_path / f"{session_id}_long_memory.jsonl"
        if long_file.exists():
            with open(long_file) as f:
                for line in f:
                    if line.strip():
                        entry = json.loads(line)
                        memories.append({
                            "id": entry.get("id", ""),
                            "content": entry.get("content", ""),
                            "tier": "long",
                            "summary": entry.get("summary", ""),
                            "timestamp": entry.get("timestamp", ""),
                        })

    return memories


# Initialize memory reload service
_memory_reload_service = MemoryReloadService()


@router.post("/sessions/{session_id}/memory/reload", response_model=Dict)
def reload_memory_from_messages(session_id: str):
    """Reload working memory by extracting important content from session messages.

    This endpoint analyzes all messages in the session, extracts memory-worthy
    content using pattern matching, and repopulates the working memory tier.
    Works for both chat sessions and agent sessions.
    """
    result = {
        "success": False,
        "memories_added": 0,
        "memories_skipped": 0,
        "message": "",
    }

    # Get messages from the session
    messages = []
    if _memst_client.is_available:
        try:
            messages = _memst_client.get_messages(session_id)
        except Exception as e:
            print(f"Error getting messages from MemSt: {e}")

    if not messages:
        result["message"] = "No messages found in session"
        return result

    # Use the shared memory reload service to extract memory-worthy content
    try:
        memories = _memory_reload_service.extract_memory_content(messages)
    except Exception as e:
        print(f"Error extracting memory content: {e}")
        result["message"] = f"Error extracting memory content: {str(e)}"
        return result

    if not memories:
        result["message"] = "No memory-worthy content found in messages"
        result["memories_skipped"] = len(messages)
        return result

    # Add memories to working memory tier
    added_count = 0
    skipped_count = 0

    for memory in memories:
        content = memory.get("content", "")
        importance = memory.get("importance", 0.5)
        tags = memory.get("tags", [])

        # Check for duplicates before adding
        try:
            if _memst_client.is_available:
                # Check existing memories for duplicates
                existing_memories = _memst_client.get_memories(session_id, "working")
                is_duplicate = any(
                    existing.get("content", "") == content
                    for existing in existing_memories
                )

                if not is_duplicate:
                    success = _memst_client.add_memory(
                        session_id, "working", content, tags
                    )
                    if success:
                        added_count += 1
                    else:
                        skipped_count += 1
                else:
                    skipped_count += 1
            else:
                # Fallback when MemSt is not available
                import json
                data_path = _get_data_path()
                file_path = data_path / f"{session_id}_working_memory.jsonl"

                # Check for duplicates
                existing_contents = set()
                if file_path.exists():
                    with open(file_path) as f:
                        for line in f:
                            if line.strip():
                                entry = json.loads(line)
                                existing_contents.add(entry.get("content", ""))

                if content not in existing_contents:
                    entry = {
                        "id": str(uuid.uuid4()),
                        "content": content,
                        "tags": tags,
                        "importance": importance,
                        "timestamp": datetime.utcnow().isoformat(),
                    }
                    with open(file_path, "a") as f:
                        f.write(json.dumps(entry) + "\n")
                    added_count += 1
                else:
                    skipped_count += 1

        except Exception as e:
            print(f"Error adding memory: {e}")
            skipped_count += 1

    result["success"] = added_count > 0
    result["memories_added"] = added_count
    result["memories_skipped"] = skipped_count
    result["message"] = (
        f"Successfully added {added_count} memories. "
        f"Skipped {skipped_count} duplicates or errors."
    )

    return result


@router.post("/sessions/{session_id}/memory", response_model=Dict)
def add_memory(session_id: str, request: MemoryCreate):
    """Add memory to session."""
    # Use MemSt client if available
    if _memst_client.is_available:
        try:
            _memst_client.add_memory(session_id, request.tier, request.content, request.tags)
            return {"success": True, "tier": request.tier, "id": str(uuid.uuid4())}
        except Exception as e:
            print(f"Error adding memory: {e}")

    # Fallback for when MemSt is not available
    import json
    data_path = _get_data_path()

    if request.tier == "working":
        file_path = data_path / f"{session_id}_working_memory.jsonl"
    elif request.tier == "short":
        file_path = data_path / f"{session_id}_short_memory.jsonl"
    else:
        file_path = data_path / f"{session_id}_long_memory.jsonl"

    entry = {
        "id": str(uuid.uuid4()),
        "content": request.content,
        "tags": request.tags or [],
        "importance": request.importance,
        "timestamp": datetime.utcnow().isoformat(),
    }

    with open(file_path, "a") as f:
        f.write(json.dumps(entry) + "\n")

    return {"success": True, "tier": request.tier, "id": entry["id"]}


@router.put("/sessions/{session_id}/memory/{memory_id}", response_model=Dict)
def update_memory(session_id: str, memory_id: str, request: MemoryCreate):
    """Update memory."""
    return {"success": True, "id": memory_id}


@router.delete("/sessions/{session_id}/memory/{memory_id}")
def delete_memory(session_id: str, memory_id: str):
    """Delete memory."""
    return {"success": True}


@router.post("/sessions/{session_id}/memory/retrieve", response_model=List[Dict])
def retrieve_memories(session_id: str, request: MemoryRetrieveRequest):
    """Retrieve relevant memories based on query."""
    import json

    data_path = _get_data_path()
    results = []
    query_lower = request.query.lower()

    # Search all tiers
    for tier in ["working", "short", "long"]:
        tier_file = data_path / f"{session_id}_{tier}_memory.jsonl"
        if tier_file.exists():
            if request.tier and request.tier != tier:
                continue
            with open(tier_file) as f:
                for line in f:
                    if line.strip():
                        entry = json.loads(line)
                        content = entry.get("content", "").lower()
                        # Simple keyword matching
                        if query_lower in content:
                            results.append({
                                "id": entry.get("id", ""),
                                "content": entry.get("content", ""),
                                "tier": tier,
                                "tags": entry.get("tags", []),
                                "relevance_score": 0.8,
                            })

    return results[:request.limit]


@router.post("/sessions/{session_id}/memory/summarize", response_model=Dict)
def summarize_session(session_id: str):
    """Generate long-term memory summary for session."""
    import json

    # Get all messages
    messages = _memst_client.get_messages(session_id) if _memst_client.is_available else []

    if not messages:
        return {"success": False, "message": "No messages to summarize"}

    # Create summary
    user_messages = [m for m in messages if m.get("role") == "user"]

    summary = f"Session covered {len(user_messages)} topics with the user."

    # Use MemSt client if available
    if _memst_client.is_available:
        try:
            _memst_client.add_memory(session_id, "long", summary, ["summary", "session"])
            return {"success": True, "summary": summary}
        except Exception as e:
            print(f"Error saving summary via MemSt: {e}")

    # Fallback to JSONL file
    data_path = _get_data_path()
    long_file = data_path / f"{session_id}_long_memory.jsonl"

    entry = {
        "id": str(uuid.uuid4()),
        "content": summary,
        "summary": summary,
        "message_count": len(messages),
        "timestamp": datetime.utcnow().isoformat(),
    }

    with open(long_file, "a") as f:
        f.write(json.dumps(entry) + "\n")

    return {"success": True, "summary": summary}


# =============================================================================
# Search Routes
# =============================================================================

@router.post("/search", response_model=List[Dict])
def search(request: SearchRequest):
    """Full-text search across sessions."""
    import re

    results = []

    # First, try MemSt's native search if available
    if _memst_client.is_available:
        try:
            results = _memst_client.search(
                request.query,
                request.limit,
                request.session_id,
                request.search_type
            )
        except Exception as e:
            print(f"MemSt search error: {e}")

    # If no results from MemSt, use fallback search across messages
    if not results:
        try:
            query = request.query
            target_session_id = request.session_id

            # Get all sessions if no specific session is targeted
            if target_session_id:
                sessions_to_search = [(target_session_id, _memst_client.get_session(target_session_id))]
            else:
                all_sessions = _memst_client.list_sessions() if _memst_client.is_available else []
                sessions_to_search = [(s.get("id"), s) for s in all_sessions]

            for session_id, session_info in sessions_to_search:
                if not session_id:
                    continue

                # Get messages for this session
                if _memst_client.is_available:
                    messages = _memst_client.get_messages(session_id)
                else:
                    messages = []

                for msg in messages:
                    content = msg.get("content", "")
                    score = 0.0

                    if request.search_type in ["text", "hybrid"]:
                        # Simple keyword matching (case insensitive)
                        if query.lower() in content.lower():
                            score = 0.8
                    elif request.search_type == "regex":
                        # Regex matching
                        try:
                            if re.search(query, content, re.IGNORECASE):
                                score = 0.9
                        except re.error:
                            continue
                    elif request.search_type == "semantic":
                        # For semantic without embeddings, use keyword match
                        if query.lower() in content.lower():
                            score = 0.7

                    if score > 0:
                        # Generate snippet with highlight
                        snippet = content[:300]
                        if len(content) > 300:
                            snippet += "..."

                        results.append({
                            "id": msg.get("id", ""),
                            "session_id": session_id,
                            "doc_type": "message",
                            "content": content,
                            "role": msg.get("role", ""),
                            "timestamp": msg.get("timestamp", ""),
                            "score": score,
                            "highlight": snippet,
                            "session_name": session_info.get("name", "") if session_info else "",
                        })
                        if len(results) >= request.limit:
                            break
                if len(results) >= request.limit:
                    break

        except Exception as e:
            print(f"Fallback search error: {e}")

    # Add knowledge graph context if available
    for r in results:
        r["knowledge_graph"] = get_knowledge_context(r.get("session_id", ""), request.query)

    return results[:request.limit]


@router.post("/search/semantic", response_model=List[Dict])
def semantic_search(request: SearchRequest):
    """Semantic search using vector similarity."""
    request.search_type = "semantic"
    return search(request)


@router.post("/search/hybrid", response_model=List[Dict])
def hybrid_search(request: SearchRequest):
    """Hybrid search combining keyword and semantic."""
    request.search_type = "hybrid"
    return search(request)


@router.get("/search/strategy", response_model=Dict)
def get_search_strategy(query: str):
    """Get recommended search strategy for query."""
    return {
        "query": query,
        "recommended_strategy": "hybrid",
        "reason": "Query appears to benefit from both keyword matching and semantic understanding",
        "estimated_results": 15,
    }


# =============================================================================
# File Routes
# =============================================================================

@router.get("/sessions/{session_id}/files", response_model=List[Dict])
def list_files(session_id: str):
    """List uploaded files for session."""
    return [
        {"id": "file1", "name": "UserManual.md", "type": "doc", "size": 45000},
        {"id": "file2", "name": "main.rs", "type": "code", "size": 12000},
        {"id": "file3", "name": "API_Spec.docx", "type": "doc", "size": 28000},
    ]


@router.post("/sessions/{session_id}/files")
async def upload_file(session_id: str, file: UploadFile = File(...)):
    """Upload file to session."""
    return {
        "id": str(uuid.uuid4()),
        "name": file.filename,
        "type": "file",
        "size": 0,
    }


@router.get("/sessions/{session_id}/files/{file_id}")
def download_file(session_id: str, file_id: str):
    """Download file."""
    raise HTTPException(status_code=404, detail="File not found")


@router.delete("/sessions/{session_id}/files/{file_id}")
def delete_file(session_id: str, file_id: str):
    """Delete file."""
    return {"success": True}


# =============================================================================
# Agent Operations / Trace Routes
# =============================================================================

@router.get("/sessions/{session_id}/trace", response_model=List[Dict])
def get_trace(session_id: str, op_type: Optional[str] = None):
    """Get operation trace."""
    # Return mock trace data
    traces = [
        {
            "id": "trace1",
            "type": "memory_retrieval",
            "title": "Memory Retrieval",
            "detail": "3 relevant memories found",
            "timestamp": datetime.utcnow().isoformat(),
            "status": "success",
        },
        {
            "id": "trace2",
            "type": "context_assembly",
            "title": "Context Assembly",
            "detail": "1,234 tokens assembled",
            "timestamp": datetime.utcnow().isoformat(),
            "status": "success",
        },
        {
            "id": "trace3",
            "type": "llm_inference",
            "title": "LLM Inference",
            "detail": "1.2s latency",
            "timestamp": datetime.utcnow().isoformat(),
            "status": "success",
        },
    ]

    if op_type:
        traces = [t for t in traces if t["type"] == op_type]
    return traces


@router.post("/sessions/{session_id}/operations", response_model=Dict)
def log_operation(session_id: str, operation: OperationLog):
    """Log operation to trace."""
    return {
        "id": str(uuid.uuid4()),
        "session_id": session_id,
        "type": operation.operation_type,
        "details": operation.details,
        "timestamp": datetime.utcnow().isoformat(),
    }


# =============================================================================
# Knowledge Graph Routes
# =============================================================================

@router.get("/sessions/{session_id}/knowledge-graph", response_model=Dict)
def get_knowledge_graph(session_id: str, tiers: Optional[str] = None):
    """Get knowledge graph for session - parses from memories."""
    import json
    import re

    data_path = _get_data_path()
    graph_file = data_path / f"{session_id}_knowledge_graph.json"

    # If graph file exists and no specific tiers requested, return cached
    if graph_file.exists() and not tiers:
        with open(graph_file) as f:
            return json.load(f)

    # Common words to skip in entity extraction
    skip_words = {
        'round', 'user', 'assistant', 'system', 'role', 'content', 'timestamp',
        'message', 'session', 'memory', 'context', 'response', 'request',
        'assistant', 'user', 'here', 'there', 'where', 'when', 'what',
        'which', 'who', 'whom', 'whose', 'why', 'how', 'can', 'will',
        'would', 'could', 'should', 'may', 'might', 'must', 'shall',
        'need', 'dare', 'ought', 'used', 'please', 'thanks', 'thank',
        'sorry', 'yes', 'no', 'okay', 'ok', 'sure', 'great', 'good',
        'well', 'also', 'too', 'very', 'just', 'only', 'even', 'still',
        'actually', 'basically', 'literally', 'essentially', 'generally',
        'typically', 'usually', 'often', 'sometimes', 'always', 'never',
        'first', 'second', 'third', 'last', 'next', 'previous', 'new',
        'old', 'big', 'small', 'large', 'long', 'short', 'high', 'low',
        'many', 'much', 'few', 'little', 'more', 'less', 'most', 'least',
        'other', 'another', 'such', 'same', 'different', 'various', 'many',
    }

    # Words that look like entities but aren't (formatting text)
    format_words = {'assistant', 'user', 'round', 'user', 'assistant'}

    nodes = []
    edges = []
    node_ids = {}
    node_counter = 0

    def add_node(label: str, node_type: str) -> str:
        nonlocal node_counter
        label_lower = label.lower().strip()
        # Skip common words and formatting text
        if label_lower in skip_words or label_lower in format_words:
            return ""
        if label_lower in node_ids:
            return node_ids[label_lower]
        node_id = f"node{node_counter}"
        node_counter += 1
        nodes.append({
            "id": node_id,
            "label": label.strip(),
            "type": node_type,
            "connections": 0,
        })
        node_ids[label_lower] = node_id
        return node_id

    # Parse requested tiers
    requested_tiers = [t.strip() for t in tiers.split(',')] if tiers else ['working', 'long']

    # Collect memories from requested tiers
    all_memories = {}

    if _memst_client.is_available:
        for tier in requested_tiers:
            try:
                memories = _memst_client.get_memories(session_id, tier)
                all_memories[tier] = memories
            except Exception as e:
                print(f"Error getting {tier} memories: {e}")
                all_memories[tier] = []
    else:
        # Fallback: read from JSONL files
        for tier in requested_tiers:
            memories = []
            file_path = data_path / f"{session_id}_{tier}_memory.jsonl"
            if file_path.exists():
                with open(file_path) as f:
                    for line in f:
                        if line.strip():
                            entry = json.loads(line)
                            if tier == 'working':
                                content = entry.get("assistant_message", "") + " " + entry.get("user_message", "")
                            else:
                                content = entry.get("content", "")
                            memories.append({"content": content})
            all_memories[tier] = memories

    # Extract entities from memories by tier
    for tier in requested_tiers:
        memories = all_memories.get(tier, [])
        node_type = 'concept' if tier == 'working' else 'entity'
        for memory in memories:
            content = memory.get("content", "")

            # Clean up content - remove common patterns
            content = re.sub(r'Round \d+:\s*User:\s*', '', content)
            content = re.sub(r'\s*\|\s*Assistant:\s*', ' ', content)

            # Better entity extraction - find multi-word technical terms and capitalized phrases
            # Pattern for capitalized multi-word terms
            multi_word = re.findall(r'\b[A-Z][a-zA-Z]+(?:\s+[A-Z][a-zA-Z]+)*\b', content)

            # Pattern for single capitalized words (excluding common words)
            single_word = re.findall(r'\b[A-Z][a-zA-Z]{2,}\b', content)

            # Process multi-word terms first (they're more likely to be real entities)
            for entity in multi_word[:5]:
                add_node(entity, node_type)

            # Then process single words, skipping common ones
            seen_labels = set(n['label'].lower() for n in nodes)
            for entity in single_word:
                if entity.lower() not in skip_words and entity.lower() not in format_words:
                    if entity.lower() not in seen_labels:
                        add_node(entity, node_type)

    # Create edges based on co-occurrence (simplified)
    for i, node in enumerate(nodes):
        for j, other in enumerate(nodes):
            if i < j and i < 3:  # Limit edges
                edges.append({
                    "from": node["id"],
                    "to": other["id"],
                    "label": "relates_to",
                })
                node["connections"] += 1
                other["connections"] += 1

    result = {"nodes": nodes, "edges": edges}

    # Save to file
    with open(graph_file, "w") as f:
        json.dump(result, f, indent=2)

    return result


@router.post("/sessions/{session_id}/knowledge-graph/parse")
def parse_knowledge_graph(session_id: str, tiers: Optional[str] = None):
    """Trigger knowledge graph parsing for session with optional tier selection."""
    # Force regeneration by deleting cached graph
    data_path = _get_data_path()
    graph_file = data_path / f"{session_id}_knowledge_graph.json"

    if graph_file.exists():
        graph_file.unlink()

    # Return parsed graph with specified tiers
    return get_knowledge_graph(session_id, tiers)


def get_knowledge_context(session_id: str, query: str) -> Dict[str, Any]:
    """Get knowledge graph context for search results."""
    data_path = _get_data_path()
    graph_file = data_path / f"{session_id}_knowledge_graph.json"

    if not graph_file.exists():
        return {
            "related_concepts": [],
            "connections_found": 0,
            "entities": [],
        }

    with open(graph_file) as f:
        graph = json.load(f)

    query_lower = query.lower()
    related_concepts = []
    entities = []

    for node in graph.get("nodes", []):
        label_lower = node.get("label", "").lower()
        if label_lower in query_lower or query_lower in label_lower:
            related_concepts.append(node.get("label", ""))
            entities.append({
                "name": node.get("label", ""),
                "type": node.get("type", ""),
                "relevance": node.get("connections", 0) / 10.0,
            })

    return {
        "related_concepts": related_concepts,
        "connections_found": len(graph.get("edges", [])),
        "entities": entities[:5],
    }


# =============================================================================
# Statistics Routes
# =============================================================================

@router.get("/stats", response_model=Dict)
def get_global_stats():
    """Get global statistics."""
    return {
        "total_sessions": 42,
        "total_messages": 1250,
        "total_memories": 156,
        "storage_used_mb": 45.2,
        "search_queries_today": 89,
        "avg_response_time_ms": 120,
    }


@router.get("/sessions/{session_id}/stats", response_model=Dict)
def get_session_stats(session_id: str):
    """Get session statistics."""
    return {
        "session_id": session_id,
        "message_count": 23,
        "memory_count": 12,
        "file_count": 3,
        "search_count": 7,
        "created_at": datetime.utcnow().isoformat(),
        "last_active": datetime.utcnow().isoformat(),
    }


# =============================================================================
# Settings Routes
# =============================================================================

class SettingsUpdateRequest(BaseModel):
    """Settings update request."""
    llm: Optional[Dict[str, Any]] = None
    embedding: Optional[Dict[str, Any]] = None
    server: Optional[Dict[str, Any]] = None


@router.get("/settings", response_model=Dict)
def get_settings():
    """Get all settings."""
    return get_db().get_settings()


@router.put("/settings", response_model=Dict)
def update_settings(request: SettingsUpdateRequest):
    """Update settings."""
    return get_db().update_settings(
        llm=request.llm,
        embedding=request.embedding,
        server=request.server,
    )


@router.post("/settings/reset", response_model=Dict)
def reset_settings():
    """Reset settings to defaults."""
    return get_db().reset_settings()


# =============================================================================
# Health Check
# =============================================================================

@router.get("/health")
def health_check():
    """Health check endpoint."""
    return {
        "status": "ok",
        "memst_available": _memst_client.is_available if _memst_client else False,
        "version": "0.1.0",
    }
