"""Type stubs for memst Python bindings."""

from typing import Optional, List, Dict, Any
from datetime import datetime
from enum import Enum

class Role(Enum):
    """Message role enum."""
    User = "user"
    Assistant = "assistant"
    System = "system"

class MemoryTier(Enum):
    """Memory tier enum."""
    Working = "working"
    ShortTerm = "short_term"
    LongTerm = "long_term"

class FusionStrategyWrapper(Enum):
    """Hybrid search fusion strategy."""
    Rrf = "rrf"
    Weighted = "weighted"
    Interleave = "interleave"

class SearchStrategyWrapper(Enum):
    """Search strategy."""
    Keyword = "keyword"
    Semantic = "semantic"
    Hybrid = "hybrid"

class RefTypeWrapper(Enum):
    """Git reference type."""
    Head = "head"
    Tag = "tag"
    Remote = "remote"

class SessionStore:
    """Session store for managing LLM conversation sessions."""
    
    def __init__(self, path: str) -> None:
        """Open an existing session store."""
        ...
    
    @staticmethod
    def init(path: str) -> "SessionStore":
        """Initialize a new session store."""
        ...
    
    def create_session(
        self,
        name: str = "Unnamed",
        model: str = "unknown",
        tags: Optional[List[str]] = None,
        system_prompt: Optional[str] = None
    ) -> str:
        """Create a new session. Returns session ID."""
        ...
    
    def get_session(self, session_id: str) -> Dict[str, Any]:
        """Get session metadata."""
        ...
    
    def list_sessions(self) -> List[Dict[str, Any]]:
        """List all sessions."""
        ...
    
    def delete_session(self, session_id: str) -> bool:
        """Delete a session."""
        ...
    
    def add_message(
        self,
        session_id: str,
        role: Role,
        content: str,
        name: Optional[str] = None,
        metadata: Optional[Dict[str, Any]] = None
    ) -> str:
        """Add a message to a session. Returns message ID."""
        ...
    
    def get_messages(
        self,
        session_id: str,
        start: Optional[int] = None,
        end: Optional[int] = None
    ) -> List[Dict[str, Any]]:
        """Get messages from a session."""
        ...
    
    def add_memory(
        self,
        session_id: str,
        content: str,
        tier: MemoryTier = MemoryTier.Working,
        tags: Optional[List[str]] = None,
        source: Optional[str] = None
    ) -> str:
        """Add a memory. Returns memory ID."""
        ...
    
    def get_memories(
        self,
        session_id: str,
        tier: Optional[MemoryTier] = None
    ) -> List[Dict[str, Any]]:
        """Get memories from a session."""
        ...
    
    def promote_memory(self, session_id: str, memory_id: str) -> bool:
        """Promote a memory to the next tier."""
        ...
    
    def search(
        self,
        query: str,
        session_id: Optional[str] = None,
        limit: int = 10
    ) -> List[Dict[str, Any]]:
        """Search across sessions."""
        ...

class Session:
    """A session wrapper."""
    
    @property
    def id(self) -> str: ...
    
    @property
    def name(self) -> str: ...
    
    @property
    def model(self) -> str: ...
    
    @property
    def created_at(self) -> str: ...
    
    @property
    def message_count(self) -> int: ...

class Message:
    """A message wrapper."""
    
    @property
    def id(self) -> str: ...
    
    @property
    def role(self) -> str: ...
    
    @property
    def content(self) -> str: ...
    
    @property
    def timestamp(self) -> str: ...

class MemoryItem:
    """A memory item wrapper."""
    
    @property
    def id(self) -> str: ...
    
    @property
    def content(self) -> str: ...
    
    @property
    def source(self) -> str: ...
    
    @property
    def tags(self) -> List[str]: ...
    
    @property
    def confidence(self) -> float: ...

class HnswConfigWrapper:
    """HNSW index configuration."""
    
    def __init__(
        self,
        dimension: int,
        m: int = 16,
        ef_construction: int = 200,
        ef_search: int = 100
    ) -> None: ...

class HnswIndexWrapper:
    """HNSW vector index."""
    
    def __init__(self, config: HnswConfigWrapper) -> None: ...
    
    def add_document(
        self,
        id: str,
        embedding: List[float],
        session_id: str,
        doc_type: str,
        content: str,
        timestamp: str
    ) -> None: ...
    
    def search(
        self,
        query_embedding: List[float],
        k: int = 10,
        session_id: Optional[str] = None,
        doc_type: Optional[str] = None
    ) -> List["VectorSearchResultWrapper"]: ...
    
    def delete_document(self, id: str) -> bool: ...
    
    def len(self) -> int: ...

class VectorSearchResultWrapper:
    """Vector search result."""
    
    @property
    def id(self) -> str: ...
    
    @property
    def score(self) -> float: ...
    
    @property
    def document(self) -> "DocumentInfoWrapper": ...

class DocumentInfoWrapper:
    """Document information."""
    
    @property
    def id(self) -> str: ...
    
    @property
    def session_id(self) -> str: ...
    
    @property
    def doc_type(self) -> str: ...
    
    @property
    def content(self) -> str: ...
    
    @property
    def timestamp(self) -> str: ...

class HybridSearchConfigWrapper:
    """Hybrid search configuration."""
    
    def __init__(
        self,
        keyword_weight: float = 0.5,
        semantic_weight: float = 0.5,
        fusion_strategy: FusionStrategyWrapper = FusionStrategyWrapper.Rrf,
        max_results: int = 20,
        rrf_k: int = 60
    ) -> None: ...

class HybridSearchResultWrapper:
    """Hybrid search result."""
    
    @property
    def id(self) -> str: ...
    
    @property
    def session_id(self) -> str: ...
    
    @property
    def doc_type(self) -> str: ...
    
    @property
    def content(self) -> str: ...
    
    @property
    def keyword_score(self) -> float: ...
    
    @property
    def semantic_score(self) -> float: ...
    
    @property
    def fusion_score(self) -> float: ...

class QueryRouterWrapper:
    """Query router for automatic search strategy selection."""
    
    def __init__(self, config: HybridSearchConfigWrapper) -> None: ...
    
    def route_query(self, query: str) -> SearchStrategyWrapper: ...

class ObjectIdWrapper:
    """Git-like object ID."""
    
    def __init__(self, hex: str) -> None: ...
    
    def to_hex(self) -> str: ...

class BlobWrapper:
    """Git-like blob object."""
    
    def __init__(self, data: bytes) -> None: ...
    
    @property
    def data(self) -> bytes: ...
    
    @property
    def size(self) -> int: ...

class TreeWrapper:
    """Git-like tree object."""
    
    def __init__(self, entries: List["TreeEntryWrapper"]) -> None: ...
    
    @property
    def entries(self) -> List["TreeEntryWrapper"]: ...

class TreeEntryWrapper:
    """Git-like tree entry."""
    
    def __init__(self, name: str, mode: int, oid: ObjectIdWrapper) -> None: ...
    
    @property
    def name(self) -> str: ...
    
    @property
    def mode(self) -> int: ...
    
    @property
    def oid(self) -> ObjectIdWrapper: ...

class CommitWrapper:
    """Git-like commit object."""
    
    @property
    def tree(self) -> ObjectIdWrapper: ...
    
    @property
    def parents(self) -> List[ObjectIdWrapper]: ...
    
    @property
    def author(self) -> "AuthorWrapper": ...
    
    @property
    def message(self) -> str: ...

class AuthorWrapper:
    """Git-like author."""
    
    def __init__(self, name: str, email: str, timestamp: int) -> None: ...
    
    @property
    def name(self) -> str: ...
    
    @property
    def email(self) -> str: ...
    
    @property
    def timestamp(self) -> int: ...

class TagWrapper:
    """Git-like tag object."""
    
    @property
    def target(self) -> ObjectIdWrapper: ...
    
    @property
    def name(self) -> str: ...
    
    @property
    def tagger(self) -> Optional[AuthorWrapper]: ...
    
    @property
    def message(self) -> str: ...

class MergeStrategyWrapper(Enum):
    """Merge strategy."""
    ThreeWay = "three_way"
    Ours = "ours"
    Theirs = "theirs"

class MergeResultWrapper:
    """Merge result."""
    
    @property
    def success(self) -> bool: ...
    
    @property
    def commit_id(self) -> Optional[ObjectIdWrapper]: ...
    
    @property
    def conflicts(self) -> List[str]: ...

# ============================================
# Phase 16: KG Extraction v2
# ============================================

class KgStorageWrapper:
    """KG Storage for managing extraction data."""
    
    @staticmethod
    def new(path: str) -> "KgStorageWrapper": ...
    
    @staticmethod
    def new_in_memory() -> "KgStorageWrapper": ...

class Entity:
    """Extracted entity."""
    
    @property
    def id(self) -> str: ...
    
    @property
    def doc_id(self) -> str: ...
    
    @property
    def ontology_id(self) -> str: ...
    
    @property
    def entity_type(self) -> str: ...
    
    @property
    def name(self) -> str: ...
    
    @property
    def confidence(self) -> float: ...

class ExtractionJob:
    """Extraction job result."""
    
    @property
    def id(self) -> str: ...
    
    @property
    def doc_id(self) -> str: ...
    
    @property
    def ontology_id(self) -> str: ...
    
    @property
    def status(self) -> str: ...
    
    @property
    def entity_count(self) -> int: ...
    
    @property
    def relationship_count(self) -> int: ...
    
    @property
    def tokens_used(self) -> int: ...

class ExtractionService:
    """KG Extraction Service."""
    
    def __init__(self, storage: KgStorageWrapper) -> None: ...
    
    def extract_entities(self, doc_id: str, text: str, ontology_id: str) -> ExtractionJob: ...
    
    def search_entities(self, query: str, limit: int = 10) -> List[Entity]: ...

class Ontology:
    """Ontology definition."""
    
    @property
    def id(self) -> str: ...
    
    @property
    def top_category(self) -> str: ...
    
    @property
    def first_category(self) -> str: ...
    
    @property
    def second_category(self) -> str: ...
    
    @property
    def chinese_name(self) -> str: ...
    
    @property
    def english_name(self) -> str: ...

class OntologyManager:
    """Ontology Manager."""
    
    def __init__(self) -> None: ...
    
    @staticmethod
    def from_schema_json(json_content: str) -> "OntologyManager": ...
    
    def list_all(self) -> List[Ontology]: ...
    
    def get(self, id: str) -> Optional[Ontology]: ...

__version__: str
