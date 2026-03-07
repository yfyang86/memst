#!/usr/bin/env python3
"""Tests for KG Extraction v2 Python bindings.

These tests use the config from ~/.config/memst/config.toml if available.
Set MEMST_CONFIG_PATH to override the config file location.
"""

import os
import sys
import pytest
import memst
from pathlib import Path


def load_config_path() -> Path:
    """Find the config file path.
    
    Searches in order:
    1. $MEMST_CONFIG_PATH environment variable
    2. ./config.toml
    3. ~/.config/memst/config.toml
    """
    # Check environment variable
    if config_path := os.environ.get("MEMST_CONFIG_PATH"):
        return Path(config_path)
    
    # Check current directory
    if (Path.cwd() / "config.toml").exists():
        return Path.cwd() / "config.toml"
    
    # Check home directory
    home_config = Path.home() / ".config" / "memst" / "config.toml"
    if home_config.exists():
        return home_config
    
    return None


class TestKgStorage:
    """Test KgStorage wrapper."""

    def test_new_in_memory(self):
        """Test creating in-memory storage."""
        storage = memst.KgStorage.new_in_memory()
        assert storage is not None
        assert repr(storage) == "KgStorage()"

    def test_new_file_based(self, tmp_path):
        """Test creating file-based storage."""
        db_path = str(tmp_path / "test.db")
        storage = memst.KgStorage.new(db_path)
        assert storage is not None

    def test_load_ontologies_from_schema(self):
        """Test loading ontologies from schema JSON into storage."""
        storage = memst.KgStorage.new_in_memory()
        
        schema_json = '''[
            {
                "top_category": "领域情报类",
                "first_category": "科技情报",
                "second_category": "人工智能",
                "chinese_name": "科技情报-人工智能",
                "english_name": "Tech Intelligence-AI",
                "overview": "监测AI技术发展"
            },
            {
                "top_category": "领域情报类",
                "first_category": "科技情报",
                "second_category": "半导体芯片",
                "chinese_name": "科技情报-半导体芯片",
                "english_name": "Tech Intelligence-Semiconductor",
                "overview": "追踪芯片设计、制造、封测、EDA工具全产业链"
            }
        ]'''
        
        # Load ontologies into storage
        ids = storage.load_ontologies_from_schema(schema_json)
        assert len(ids) == 2
        print(f"Loaded ontologies: {ids}")
        
        # Verify we can retrieve them
        for ontology_id in ids:
            ontology = storage.get_ontology(ontology_id)
            assert ontology is not None
            print(f"Retrieved: {ontology.english_name}")


class TestOntologyManager:
    """Test OntologyManager wrapper."""

    def test_new_empty(self):
        """Test creating empty ontology manager."""
        manager = memst.OntologyManager()
        assert manager is not None
        assert "ontologies=0" in repr(manager)

    def test_from_schema_json(self):
        """Test loading ontologies from JSON."""
        schema_json = '''[
            {
                "top_category": "领域情报类",
                "first_category": "科技情报",
                "second_category": "人工智能",
                "chinese_name": "科技情报-人工智能",
                "english_name": "Tech Intelligence-AI",
                "overview": "监测AI技术发展"
            },
            {
                "top_category": "领域情报类",
                "first_category": "科技情报",
                "second_category": "半导体芯片",
                "chinese_name": "科技情报-半导体芯片",
                "english_name": "Tech Intelligence-Semiconductor",
                "overview": "追踪芯片设计、制造、封测、EDA工具全产业链"
            }
        ]'''
        
        manager = memst.OntologyManager.from_schema_json(schema_json)
        assert manager is not None
        
        # List all ontologies
        ontologies = manager.list_all()
        assert len(ontologies) == 2
        
        # Check ontologies (order may vary)
        english_names = [ont.english_name for ont in ontologies]
        assert "Tech Intelligence-AI" in english_names
        assert "Tech Intelligence-Semiconductor" in english_names
        
        # Check first ontology structure
        ont0 = ontologies[0]
        assert ont0.top_category == "领域情报类"
        assert ont0.first_category == "科技情报"
        
        # Check repr
        assert "Ontology" in repr(ont0)

    def test_get_ontology(self):
        """Test getting ontology by ID."""
        schema_json = '''[
            {
                "top_category": "领域情报类",
                "first_category": "科技情报",
                "second_category": "人工智能",
                "chinese_name": "科技情报-人工智能",
                "english_name": "Tech Intelligence-AI",
                "overview": "监测AI技术发展"
            }
        ]'''
        
        manager = memst.OntologyManager.from_schema_json(schema_json)
        
        # Get existing ontology (ID format depends on slugify)
        # The ID is generated from top-first-second categories
        ontologies = manager.list_all()
        assert len(ontologies) > 0
        
        ontology_id = ontologies[0].id
        found = manager.get(ontology_id)
        assert found is not None
        assert found.english_name == "Tech Intelligence-AI"
        
        # Get non-existent ontology
        not_found = manager.get("non-existent-id")
        assert not_found is None


class TestExtractionService:
    """Test ExtractionService wrapper."""

    def test_new_service(self):
        """Test creating extraction service."""
        storage = memst.KgStorage.new_in_memory()
        service = memst.ExtractionService(storage)
        assert service is not None
        assert repr(service) == "ExtractionService()"

    def test_load_ontologies(self):
        """Test loading ontologies into service."""
        storage = memst.KgStorage.new_in_memory()
        service = memst.ExtractionService(storage)
        
        schema_json = '''[
            {
                "top_category": "领域情报类",
                "first_category": "科技情报",
                "second_category": "人工智能",
                "chinese_name": "科技情报-人工智能",
                "english_name": "Tech Intelligence-AI",
                "overview": "监测AI技术发展"
            }
        ]'''
        
        # Load ontologies into the service's internal storage
        ids = service.load_ontologies(schema_json)
        assert len(ids) == 1
        print(f"Loaded ontology: {ids[0]}")

    def test_extract_entities(self):
        """Test entity extraction with ontology loaded in service."""
        # Create service
        storage = memst.KgStorage.new_in_memory()
        service = memst.ExtractionService(storage)
        
        # Load ontologies into service's internal storage
        schema_json = '''[
            {
                "top_category": "领域情报类",
                "first_category": "科技情报",
                "second_category": "人工智能",
                "chinese_name": "科技情报-人工智能",
                "english_name": "Tech Intelligence-AI",
                "overview": "For testing entity extraction"
            }
        ]'''
        
        ids = service.load_ontologies(schema_json)
        assert len(ids) > 0, "Failed to load ontologies"
        ontology_id = ids[0]
        print(f"Using ontology ID: {ontology_id}")
        
        # Extract entities from text
        job = service.extract_entities(
            doc_id="doc-001",
            text="OpenAI released GPT-4 Turbo in 2023. Sam Altman is the CEO.",
            ontology_id=ontology_id
        )
        
        # Verify job result
        assert job is not None
        assert job.doc_id == "doc-001"
        assert job.ontology_id == ontology_id
        assert job.status == "completed"
        
        print(f"Extraction succeeded: {job.entity_count} entities, {job.tokens_used} tokens")

    def test_extract_entities_multiple(self):
        """Test extracting entities from multiple texts."""
        storage = memst.KgStorage.new_in_memory()
        service = memst.ExtractionService(storage)
        
        # Load ontologies
        schema_json = '''[
            {
                "top_category": "领域情报类",
                "first_category": "产业情报",
                "second_category": "企业动态",
                "chinese_name": "产业情报-企业动态",
                "english_name": "Industry-Business",
                "overview": "企业并购、融资、战略合作"
            }
        ]'''
        
        ids = service.load_ontologies(schema_json)
        assert len(ids) > 0
        ontology_id = ids[0]
        
        texts = [
            ("doc-001", "Google DeepMind announced AlphaFold 3."),
            ("doc-002", "Microsoft acquired Activision Blizzard for $68.7 billion."),
            ("doc-003", "NVIDIA and TSMC collaborate on AI chips."),
        ]
        
        for doc_id, text in texts:
            job = service.extract_entities(doc_id, text, ontology_id)
            assert job.doc_id == doc_id
            assert job.status == "completed"
            print(f"Extracted from {doc_id}: {job.entity_count} entities")

    def test_search_entities_empty(self):
        """Test searching entities (returns empty list by default)."""
        storage = memst.KgStorage.new_in_memory()
        service = memst.ExtractionService(storage)
        
        # Search returns empty list in current implementation
        results = service.search_entities("OpenAI", limit=10)
        assert isinstance(results, list)


class TestEntity:
    """Test Entity wrapper."""

    def test_entity_class_exists(self):
        """Test Entity class is available."""
        # Entity objects are returned from extraction service
        # We just verify the class exists and has expected attributes
        assert hasattr(memst.Entity, 'id')
        assert hasattr(memst.Entity, 'name')
        assert hasattr(memst.Entity, 'entity_type')
        assert hasattr(memst.Entity, 'confidence')


class TestIntegration:
    """Integration tests for KG Extraction v2."""

    def test_config_file_detection(self):
        """Test that we can detect the config file."""
        config_path = load_config_path()
        
        if config_path is None:
            pytest.skip("No config file found at ~/.config/memst/config.toml")
        
        assert config_path.exists(), f"Config file not found: {config_path}"
        print(f"Found config at: {config_path}")
        
        # Try to parse it
        try:
            import tomllib
            with open(config_path, "rb") as f:
                config = tomllib.load(f)
            
            # Check for sections
            if "llm" in config:
                print(f"LLM config: {config['llm'].get('model', 'N/A')}")
            if "kg_extraction" in config:
                print(f"KG config: {config['kg_extraction']}")
                
        except Exception as e:
            pytest.skip(f"Could not parse config: {e}")

    def test_full_workflow(self):
        """Test full extraction workflow.
        
        This test demonstrates the complete workflow:
        1. Create service
        2. Load ontologies into service from schema JSON
        3. Extract entities
        """
        # Create service
        storage = memst.KgStorage.new_in_memory()
        service = memst.ExtractionService(storage)
        
        # Load ontologies from schema
        schema_json = '''[
            {
                "top_category": "TestDomain",
                "first_category": "Integration",
                "second_category": "Workflow",
                "chinese_name": "集成测试",
                "english_name": "Integration Test",
                "overview": "For integration testing"
            }
        ]'''
        
        ids = service.load_ontologies(schema_json)
        assert len(ids) == 1
        
        ontology_id = ids[0]
        print(f"Loaded ontology: {ontology_id}")
        
        # Extract entities
        job = service.extract_entities(
            doc_id="workflow-test",
            text="Test text for extraction workflow.",
            ontology_id=ontology_id
        )
        
        # Verify extraction completed
        assert job.status == "completed"
        print(f"Workflow test passed: {job.entity_count} entities, {job.tokens_used} tokens")

    def test_ontology_repr(self):
        """Test ontology string representation."""
        schema_json = '''[
            {
                "top_category": "A",
                "first_category": "B",
                "second_category": "C",
                "chinese_name": "中文",
                "english_name": "English Name",
                "overview": "Overview"
            }
        ]'''
        
        manager = memst.OntologyManager.from_schema_json(schema_json)
        ontology = manager.list_all()[0]
        
        repr_str = repr(ontology)
        assert "Ontology" in repr_str
        assert ontology.id in repr_str
        assert "English Name" in repr_str


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
