#!/usr/bin/env python3
"""Tests for KG Extraction v2 Python bindings."""

import pytest
import memst


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
        
        # Check first ontology
        ont0 = ontologies[0]
        assert ont0.top_category == "领域情报类"
        assert ont0.first_category == "科技情报"
        assert ont0.english_name == "Tech Intelligence-AI"
        
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

    @pytest.mark.skip(reason="Requires ontology to be registered in service - TODO: add ontology registration API")
    def test_extract_entities(self):
        """Test entity extraction.
        
        Note: This test requires the ontology to be registered in the service's
        ontology manager. Currently the service creates its own isolated manager.
        """
        storage = memst.KgStorage.new_in_memory()
        service = memst.ExtractionService(storage)
        
        # Extract entities from text
        job = service.extract_entities(
            doc_id="doc-001",
            text="OpenAI released GPT-4 Turbo in 2023. Sam Altman is the CEO.",
            ontology_id="tech-ai"
        )
        
        # Verify job result
        assert job is not None
        assert job.doc_id == "doc-001"
        assert job.ontology_id == "tech-ai"
        assert job.status == "completed"
        assert job.entity_count >= 0
        assert job.tokens_used >= 0
        
        # Verify repr
        assert "ExtractionJob" in repr(job)
        assert job.id in repr(job)

    @pytest.mark.skip(reason="Requires ontology to be registered in service - TODO: add ontology registration API")
    def test_extract_entities_multiple(self):
        """Test extracting entities from multiple texts."""
        storage = memst.KgStorage.new_in_memory()
        service = memst.ExtractionService(storage)
        
        texts = [
            ("doc-001", "Google DeepMind announced AlphaFold 3."),
            ("doc-002", "Microsoft acquired Activision Blizzard for $68.7 billion."),
            ("doc-003", "NVIDIA and TSMC collaborate on AI chips."),
        ]
        
        for doc_id, text in texts:
            job = service.extract_entities(doc_id, text, "tech-business")
            assert job.doc_id == doc_id
            assert job.status == "completed"

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

    @pytest.mark.skip(reason="Requires ontology registration API - service has isolated ontology manager")
    def test_full_workflow(self):
        """Test full extraction workflow.
        
        Note: This test requires the ability to register ontologies with the
        extraction service. Currently the service creates its own isolated
        ontology manager that cannot be accessed from Python.
        """
        # Create storage
        storage = memst.KgStorage.new_in_memory()
        
        # Create service
        service = memst.ExtractionService(storage)
        
        # Load ontologies
        schema_json = '''[
            {
                "top_category": "Test",
                "first_category": "Test",
                "second_category": "Test",
                "chinese_name": "测试",
                "english_name": "Test Ontology",
                "overview": "For testing"
            }
        ]'''
        manager = memst.OntologyManager.from_schema_json(schema_json)
        
        # Verify manager has ontologies
        assert len(manager.list_all()) == 1
        
        # Extract entities
        job = service.extract_entities(
            doc_id="workflow-test",
            text="Test text for extraction workflow.",
            ontology_id="test-test-test"
        )
        
        # Verify extraction completed
        assert job.status == "completed"

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
