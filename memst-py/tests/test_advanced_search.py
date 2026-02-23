import pytest

from memst import HnswConfig, HnswIndex, QueryRouter


class TestHnswIndex:
    def test_hnsw_add_search_delete(self):
        cfg = HnswConfig()
        cfg.similarity_threshold = -1.0
        cfg.ef_construction = max(cfg.ef_construction, 64)
        cfg.ef_search = max(cfg.ef_search, 64)

        index = HnswIndex(2, cfg)
        assert index.dimension() == 2
        assert index.len() == 0

        index.add_document(
            "doc-a",
            [1.0, 0.0],
            session_id="s1",
            doc_type="message",
            content="hello from a",
        )
        index.add_document(
            "doc-b",
            [0.0, 1.0],
            session_id="s2",
            doc_type="message",
            content="hello from b",
        )
        assert index.len() == 2

        results = index.search([1.0, 0.0], limit=1)
        assert len(results) == 1
        assert results[0].id == "doc-a"
        assert results[0].document.id == results[0].id

        results_b = index.search([0.0, 1.0], limit=1)
        assert len(results_b) == 1
        assert results_b[0].id == "doc-b"

        # Cover the wrapper method (core filtering is currently a no-op).
        filtered = index.search_filtered([1.0, 0.0], limit=2, session_filter="s1")
        assert isinstance(filtered, list)

        assert index.delete("doc-a") is True
        assert index.delete("doc-a") is False
        assert index.len() == 1

    def test_hnsw_repr_smoke(self):
        index = HnswIndex(3)
        text = repr(index)
        assert "HnswIndex" in text
        assert "dimension=3" in text


class TestQueryRouter:
    def test_analyze_query_returns_known_strategy(self):
        router = QueryRouter()
        strategy = router.analyze_query("find messages about rust ownership")
        assert strategy in {"Keyword", "Semantic", "Hybrid"}

    def test_explain_recommendation_valid_and_invalid_strategy(self):
        router = QueryRouter()
        explanation = router.explain_recommendation(
            "find messages about rust ownership",
            "hybrid",
        )
        assert isinstance(explanation, str)
        assert explanation.strip()

        with pytest.raises(ValueError):
            router.explain_recommendation("q", "not-a-strategy")
