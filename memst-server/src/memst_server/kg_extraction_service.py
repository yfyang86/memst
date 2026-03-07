"""KG Extraction v2 service for MemSt server.

This module provides Knowledge Graph extraction capabilities using the
memst-extract-v2 Rust library through Python bindings.
"""

import json
from pathlib import Path
from typing import Optional, List, Dict, Any
from datetime import datetime

# Import memst for KG extraction v2
try:
    import memst
    MEMST_AVAILABLE = True
except ImportError:
    MEMST_AVAILABLE = False
    print("Warning: memst module not available. KG Extraction v2 disabled.")


class KGExtractionService:
    """Knowledge Graph extraction service using KG Extraction v2."""
    
    def __init__(self, db_path: Optional[str] = None):
        """Initialize KG extraction service.
        
        Args:
            db_path: Path to SQLite database for KG storage. If None, uses in-memory.
        """
        self._storage = None
        self._service = None
        self._db_path = db_path
        self._ontologies: Dict[str, Any] = {}
        
        if MEMST_AVAILABLE:
            self._init_service()
    
    def _init_service(self):
        """Initialize the extraction service."""
        try:
            if self._db_path:
                self._storage = memst.KgStorage.new(self._db_path)
            else:
                self._storage = memst.KgStorage.new_in_memory()
            
            self._service = memst.ExtractionService(self._storage)
            print(f"KG Extraction v2 initialized (db: {self._db_path or 'in-memory'})")
        except Exception as e:
            print(f"Warning: Failed to initialize KG Extraction service: {e}")
            self._storage = None
            self._service = None
    
    @property
    def is_available(self) -> bool:
        """Check if KG extraction is available."""
        return MEMST_AVAILABLE and self._service is not None
    
    def load_ontologies_from_schema(self, schema_json: str) -> List[str]:
        """Load ontologies from schema JSON.
        
        Args:
            schema_json: JSON string containing ontology schema entries
            
        Returns:
            List of loaded ontology IDs
        """
        if not self.is_available:
            return []
        
        try:
            # Load into service's internal storage
            ids = self._service.load_ontologies(schema_json)
            
            # Parse and store ontology info
            schema_data = json.loads(schema_json)
            for entry in schema_data:
                if isinstance(entry, dict):
                    ont_id = self._generate_ontology_id(entry)
                    self._ontologies[ont_id] = {
                        "id": ont_id,
                        "top_category": entry.get("top_category", ""),
                        "first_category": entry.get("first_category", ""),
                        "second_category": entry.get("second_category", ""),
                        "chinese_name": entry.get("chinese_name", ""),
                        "english_name": entry.get("english_name", ""),
                        "overview": entry.get("overview", ""),
                    }
            
            return ids
        except Exception as e:
            print(f"Error loading ontologies: {e}")
            return []
    
    def _generate_ontology_id(self, entry: Dict[str, str]) -> str:
        """Generate ontology ID from entry."""
        # Simple slugify matching Rust implementation
        def slugify(s: str) -> str:
            return s.lower().replace(" ", "-").replace("/", "-")
        
        return f"{slugify(entry.get('top_category', ''))}-{slugify(entry.get('first_category', ''))}-{slugify(entry.get('second_category', ''))}"
    
    def extract_entities(
        self,
        doc_id: str,
        text: str,
        ontology_id: str,
    ) -> Optional[Dict[str, Any]]:
        """Extract entities from text.
        
        Args:
            doc_id: Document ID
            text: Text to extract entities from
            ontology_id: Ontology ID to use for extraction
            
        Returns:
            Extraction job result
        """
        if not self.is_available:
            return None
        
        try:
            job = self._service.extract_entities(doc_id, text, ontology_id)
            return {
                "id": job.id,
                "doc_id": job.doc_id,
                "ontology_id": job.ontology_id,
                "status": job.status,
                "entity_count": job.entity_count,
                "relationship_count": job.relationship_count,
                "tokens_used": job.tokens_used,
            }
        except Exception as e:
            print(f"Error extracting entities: {e}")
            return None
    
    def search_entities(self, query: str, limit: int = 10) -> List[Dict[str, Any]]:
        """Search entities by name.
        
        Args:
            query: Search query
            limit: Maximum results
            
        Returns:
            List of matching entities
        """
        if not self.is_available:
            return []
        
        try:
            entities = self._service.search_entities(query, limit)
            return [
                {
                    "id": e.id,
                    "name": e.name,
                    "entity_type": e.entity_type,
                    "confidence": e.confidence,
                }
                for e in entities
            ]
        except Exception as e:
            print(f"Error searching entities: {e}")
            return []
    
    def get_ontology(self, ontology_id: str) -> Optional[Dict[str, Any]]:
        """Get ontology by ID.
        
        Args:
            ontology_id: Ontology ID
            
        Returns:
            Ontology data if found
        """
        # Check cached ontologies
        if ontology_id in self._ontologies:
            return self._ontologies[ontology_id]
        
        # Try to get from storage
        if self.is_available and self._storage:
            try:
                ont = self._storage.get_ontology(ontology_id)
                if ont:
                    return {
                        "id": ont.id,
                        "top_category": ont.top_category,
                        "first_category": ont.first_category,
                        "second_category": ont.second_category,
                        "chinese_name": ont.chinese_name,
                        "english_name": ont.english_name,
                    }
            except Exception as e:
                print(f"Error getting ontology: {e}")
        
        return None
    
    def list_ontologies(self) -> List[Dict[str, Any]]:
        """List all loaded ontologies.
        
        Returns:
            List of ontology metadata
        """
        return list(self._ontologies.values())
    
    def extract_from_session_messages(
        self,
        session_id: str,
        messages: List[Dict[str, Any]],
        ontology_id: str,
    ) -> Optional[Dict[str, Any]]:
        """Extract entities from session messages.
        
        Args:
            session_id: Session ID
            messages: List of messages to extract from
            ontology_id: Ontology ID to use
            
        Returns:
            Combined extraction results
        """
        if not self.is_available:
            return None
        
        all_entities = []
        total_tokens = 0
        
        for i, msg in enumerate(messages):
            content = msg.get("content", "")
            if not content:
                continue
            
            doc_id = f"{session_id}-msg-{i}"
            result = self.extract_entities(doc_id, content, ontology_id)
            
            if result:
                total_tokens += result.get("tokens_used", 0)
                # Note: entities would be stored in DB, we return summary
        
        return {
            "session_id": session_id,
            "messages_processed": len(messages),
            "total_tokens_used": total_tokens,
            "status": "completed",
        }


# Global service instance
_kg_service: Optional[KGExtractionService] = None


def get_kg_extraction_service(db_path: Optional[str] = None) -> KGExtractionService:
    """Get global KG extraction service instance.
    
    Args:
        db_path: Optional path to database
        
    Returns:
        KGExtractionService instance
    """
    global _kg_service
    if _kg_service is None:
        _kg_service = KGExtractionService(db_path)
    return _kg_service
