"""
Advanced benchmark strategies for MemSt.
Implements page-aware RAG, hierarchical retrieval, and hybrid combinations.
"""
import sys
sys.path.insert(0, str(__file__).rsplit('/', 2)[0])

import re
import json
import time
import statistics
from typing import Dict, Any, List, Tuple, Optional
from collections import defaultdict
from dataclasses import dataclass
import numpy as np

try:
    from strategies.base import BenchmarkStrategy
except ImportError:
    from base import BenchmarkStrategy


@dataclass
class Page:
    """A page of conversation (sliding window)."""
    id: int
    text: str
    start_turn: int
    end_turn: int
    session_id: str
    timestamp: str
    
    @property
    def tokens(self) -> int:
        return len(self.text) // 4


@dataclass
class SessionSummary:
    """Summary of a session."""
    session_id: str
    summary: str
    key_facts: List[str]
    entities: List[str]
    timestamp: str


class PageAwareRAGStrategy(BenchmarkStrategy):
    """
    Page-aware RAG with sliding windows.
    Divides conversation into overlapping pages for better context preservation.
    """
    
    def __init__(
        self,
        page_size: int = 512,  # tokens
        overlap: int = 128,    # tokens
        top_k: int = 3
    ):
        super().__init__(f"PageRAG-{page_size}-k{top_k}")
        self.page_size = page_size
        self.overlap = overlap
        self.top_k = top_k
        self.pages: List[Page] = []
        self.page_embeddings = []
    
    def setup(self, conversation) -> None:
        """Build page index from conversation."""
        turns = conversation.get_all_turns()
        session_dates = conversation.session_dates
        
        current_page_text = []
        current_tokens = 0
        page_id = 0
        start_turn = 0
        current_session = "session_1"
        
        for i, turn in enumerate(turns):
            turn_text = f"{turn['speaker']}: {turn['text']}"
            turn_tokens = len(turn_text) // 4
            
            # Detect session change
            for j, date in enumerate(session_dates[:i//20 + 1], 1):
                if i < j * 20:  # Rough session boundary
                    current_session = f"session_{j}"
                    break
            
            if current_tokens + turn_tokens > self.page_size:
                # Save current page
                page = Page(
                    id=page_id,
                    text="\n".join(current_page_text),
                    start_turn=start_turn,
                    end_turn=i,
                    session_id=current_session,
                    timestamp=session_dates[min(len(session_dates)-1, page_id)] if session_dates else ""
                )
                self.pages.append(page)
                
                # Start new page with overlap
                overlap_text = current_page_text[-self.overlap//50:] if len(current_page_text) > 1 else []
                current_page_text = overlap_text + [turn_text]
                current_tokens = sum(len(t) for t in current_page_text) // 4
                start_turn = i - len(overlap_text)
                page_id += 1
            else:
                current_page_text.append(turn_text)
                current_tokens += turn_tokens
        
        # Add final page
        if current_page_text:
            page = Page(
                id=page_id,
                text="\n".join(current_page_text),
                start_turn=start_turn,
                end_turn=len(turns),
                session_id=current_session,
                timestamp=session_dates[-1] if session_dates else ""
            )
            self.pages.append(page)
        
        # Compute embeddings (simplified - would use real embeddings)
        self.page_embeddings = self._compute_embeddings([p.text for p in self.pages])
    
    def _compute_embeddings(self, texts: List[str]) -> List[List[float]]:
        """Compute simple bag-of-words embeddings."""
        embeddings = []
        for text in texts:
            words = set(re.findall(r'\b[a-zA-Z]+\b', text.lower()))
            # Create a simple hash-based embedding
            emb = [hash(w) % 1000 / 1000.0 for w in list(words)[:50]]
            emb += [0.0] * (50 - len(emb))  # Pad to fixed length
            embeddings.append(emb)
        return embeddings
    
    def _compute_similarity(self, emb1: List[float], emb2: List[float]) -> float:
        """Cosine similarity."""
        a = np.array(emb1)
        b = np.array(emb2)
        return np.dot(a, b) / (np.linalg.norm(a) * np.linalg.norm(b) + 1e-8)
    
    def answer(self, question) -> Dict[str, Any]:
        """Retrieve top-k pages."""
        start_time = time.time()
        
        # Compute query embedding
        query_emb = self._compute_embeddings([question.question])[0]
        
        # Score all pages
        scores = []
        for i, page_emb in enumerate(self.page_embeddings):
            score = self._compute_similarity(query_emb, page_emb)
            # Boost recent pages
            recency_boost = 1.0 + (i / len(self.pages)) * 0.2
            scores.append((i, score * recency_boost))
        
        # Get top-k
        top_pages = sorted(scores, key=lambda x: -x[1])[:self.top_k]
        
        # Build context
        context_parts = []
        total_tokens = 0
        for page_id, score in top_pages:
            page = self.pages[page_id]
            context_parts.append(f"[Page {page.id} - {page.session_id}]\n{page.text}")
            total_tokens += page.tokens
        
        context = "\n\n".join(context_parts)
        latency = time.time() - start_time
        
        result = {
            'answer': "",
            'context': context,
            'tokens': total_tokens,
            'latency': latency,
            'pages_retrieved': len(top_pages)
        }
        
        self.metrics['total_queries'] += 1
        self.metrics['total_time'] += latency
        self.metrics['tokens_retrieved'] += total_tokens
        
        return result


class HierarchicalSummaryStrategy(BenchmarkStrategy):
    """
    Hierarchical retrieval with session summaries.
    Level 1: Session summaries
    Level 2: Key turns
    Level 3: Full context
    """
    
    def __init__(
        self,
        summary_budget: int = 0.3,  # 30% of tokens for summaries
        key_turn_budget: int = 0.4,  # 40% for key turns
        total_budget: int = 2048
    ):
        super().__init__("Hierarchical-Summary")
        self.summary_budget = summary_budget
        self.key_turn_budget = key_turn_budget
        self.total_budget = total_budget
        self.summaries: List[SessionSummary] = []
        self.key_turns: Dict[str, List[str]] = {}
        self.full_text = ""
    
    def _generate_summary(self, session_turns: List[Dict], session_id: str) -> SessionSummary:
        """Generate simple extractive summary."""
        # Extract key sentences (containing named entities or important words)
        key_facts = []
        entities = set()
        
        for turn in session_turns:
            text = turn['text']
            # Extract potential entities (capitalized words)
            caps = re.findall(r'\b[A-Z][a-z]+\s+[A-Z][a-z]+\b|\b[A-Z][a-z]+\b', text)
            entities.update(caps)
            
            # Extract sentences with temporal markers
            if re.search(r'\b(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec|January|February|March|April|May|June|July|August|September|October|November|December|on\s+(Monday|Tuesday|Wednesday|Thursday|Friday|Saturday|Sunday)|last\s+(week|month|year)|in\s+\d{4})\b', text, re.IGNORECASE):
                key_facts.append(text[:200])  # Truncate long facts
        
        summary = f"Session {session_id}: " + "; ".join(key_facts[:3])
        
        return SessionSummary(
            session_id=session_id,
            summary=summary,
            key_facts=key_facts[:5],
            entities=list(entities)[:10],
            timestamp=""
        )
    
    def setup(self, conversation) -> None:
        """Build hierarchical index."""
        self.full_text = conversation.get_full_text()
        
        # Generate session summaries
        session_num = 1
        while f"session_{session_num}" in conversation.sessions:
            session_turns = conversation.sessions[session_num - 1]
            summary = self._generate_summary(session_turns, f"session_{session_num}")
            self.summaries.append(summary)
            
            # Extract key turns (first, last, and turns with entities)
            key = []
            if session_turns:
                key.append(session_turns[0]['text'])  # First turn
                key.append(session_turns[-1]['text'])  # Last turn
                # Turns with temporal markers
                for turn in session_turns[1:-1]:
                    if re.search(r'\b(when|date|time|year|month|day)\b', turn['text'], re.IGNORECASE):
                        key.append(turn['text'])
            self.key_turns[f"session_{session_num}"] = key[:5]
            
            session_num += 1
    
    def answer(self, question) -> Dict[str, Any]:
        """Hierarchical retrieval based on question type."""
        start_time = time.time()
        
        context_parts = []
        total_tokens = 0
        
        # Strategy based on question category
        if question.category == 1:  # Single-hop
            # Use key turns from relevant sessions
            for session_id, turns in self.key_turns.items():
                for turn in turns[:2]:
                    turn_tokens = len(turn) // 4
                    if total_tokens + turn_tokens < self.total_budget * self.key_turn_budget:
                        context_parts.append(turn)
                        total_tokens += turn_tokens
        
        elif question.category in [2, 3]:  # Multi-hop or Temporal
            # Use summaries + key turns
            for summary in self.summaries:
                summary_tokens = len(summary.summary) // 4
                if total_tokens + summary_tokens < self.total_budget * self.summary_budget:
                    context_parts.append(summary.summary)
                    total_tokens += summary_tokens
            
            # Add key facts for temporal
            if question.category == 3:
                for summary in self.summaries:
                    for fact in summary.key_facts:
                        fact_tokens = len(fact) // 4
                        if total_tokens + fact_tokens < self.total_budget * 0.2:
                            context_parts.append(fact)
                            total_tokens += fact_tokens
        
        else:  # Open-domain or Adversarial
            # Use summaries only
            for summary in self.summaries:
                summary_tokens = len(summary.summary) // 4
                if total_tokens + summary_tokens < self.total_budget:
                    context_parts.append(summary.summary)
                    total_tokens += summary_tokens
        
        context = "\n".join(context_parts)
        latency = time.time() - start_time
        
        result = {
            'answer': "",
            'context': context,
            'tokens': total_tokens,
            'latency': latency
        }
        
        self.metrics['total_queries'] += 1
        self.metrics['total_time'] += latency
        self.metrics['tokens_retrieved'] += total_tokens
        
        return result


class WeightedRRFStrategy(BenchmarkStrategy):
    """
    Weighted Reciprocal Rank Fusion with category-specific weights.
    """
    
    def __init__(
        self,
        weights: Dict[int, Tuple[float, float]] = None,  # category -> (bm25_weight, vector_weight)
        rrf_k: int = 20
    ):
        super().__init__("WeightedRRF")
        self.weights = weights or {
            1: (0.7, 0.3),  # Single-hop: more BM25
            2: (0.5, 0.5),  # Multi-hop: balanced
            3: (0.4, 0.6),  # Temporal: more semantic
            4: (0.3, 0.7),  # Open-domain: more semantic
            5: (0.5, 0.5),  # Adversarial: balanced
        }
        self.rrf_k = rrf_k
        self.candidates = []
    
    def setup(self, conversation) -> None:
        """Build candidate pool."""
        turns = conversation.get_all_turns()
        self.candidates = [
            {
                'text': f"{t['speaker']}: {t['text']}",
                'index': i,
                'session': f"session_{i//20 + 1}"
            }
            for i, t in enumerate(turns)
        ]
    
    def _bm25_rank(self, query: str) -> List[Tuple[int, float]]:
        """Simple BM25 ranking."""
        query_terms = set(re.findall(r'\b[a-zA-Z]+\b', query.lower()))
        scores = []
        
        for i, cand in enumerate(self.candidates):
            text_terms = set(re.findall(r'\b[a-zA-Z]+\b', cand['text'].lower()))
            overlap = len(query_terms & text_terms)
            score = overlap / (len(query_terms) + 1)
            scores.append((i, score))
        
        return sorted(scores, key=lambda x: -x[1])
    
    def _vector_rank(self, query: str) -> List[Tuple[int, float]]:
        """Simple vector ranking (hash-based)."""
        query_emb = self._text_to_emb(query)
        scores = []
        
        for i, cand in enumerate(self.candidates):
            cand_emb = self._text_to_emb(cand['text'])
            sim = np.dot(query_emb, cand_emb) / (np.linalg.norm(query_emb) * np.linalg.norm(cand_emb) + 1e-8)
            scores.append((i, sim))
        
        return sorted(scores, key=lambda x: -x[1])
    
    def _text_to_emb(self, text: str) -> np.ndarray:
        """Simple embedding."""
        words = list(set(re.findall(r'\b[a-zA-Z]+\b', text.lower())))[:20]
        emb = np.array([hash(w) % 100 / 100.0 for w in words] + [0.0] * (20 - len(words)))
        return emb
    
    def answer(self, question) -> Dict[str, Any]:
        """Weighted RRF retrieval."""
        start_time = time.time()
        
        # Get rankings
        bm25_ranks = self._bm25_rank(question.question)
        vec_ranks = self._vector_rank(question.question)
        
        # Create rank dictionaries
        bm25_rank_dict = {idx: rank+1 for rank, (idx, _) in enumerate(bm25_ranks)}
        vec_rank_dict = {idx: rank+1 for rank, (idx, _) in enumerate(vec_ranks)}
        
        # Get weights for category
        w_bm25, w_vec = self.weights.get(question.category, (0.5, 0.5))
        
        # Compute weighted RRF
        rrf_scores = {}
        all_ids = set(bm25_rank_dict.keys()) | set(vec_rank_dict.keys())
        
        for idx in all_ids:
            score = 0
            if idx in bm25_rank_dict:
                score += w_bm25 / (self.rrf_k + bm25_rank_dict[idx])
            if idx in vec_rank_dict:
                score += w_vec / (self.rrf_k + vec_rank_dict[idx])
            rrf_scores[idx] = score
        
        # Get top candidates
        top_ids = sorted(rrf_scores.items(), key=lambda x: -x[1])[:10]
        
        # Build context
        context_parts = []
        total_tokens = 0
        for idx, score in top_ids:
            text = self.candidates[idx]['text']
            context_parts.append(text)
            total_tokens += len(text) // 4
        
        context = "\n".join(context_parts)
        latency = time.time() - start_time
        
        result = {
            'answer': "",
            'context': context,
            'tokens': total_tokens,
            'latency': latency
        }
        
        self.metrics['total_queries'] += 1
        self.metrics['total_time'] += latency
        self.metrics['tokens_retrieved'] += total_tokens
        
        return result


class CascadeRetrievalStrategy(BenchmarkStrategy):
    """
    Cascade retrieval: BM25 -> Vector -> Cross-encoder.
    """
    
    def __init__(
        self,
        stage1_top_k: int = 50,
        stage2_top_k: int = 10,
        final_top_k: int = 5
    ):
        super().__init__(f"Cascade-{stage1_top_k}-{stage2_top_k}-{final_top_k}")
        self.stage1_top_k = stage1_top_k
        self.stage2_top_k = stage2_top_k
        self.final_top_k = final_top_k
        self.candidates = []
    
    def setup(self, conversation) -> None:
        """Build candidate pool."""
        turns = conversation.get_all_turns()
        self.candidates = [
            {
                'text': f"{t['speaker']}: {t['text']}",
                'index': i,
                'embedding': self._quick_embed(t['text'])
            }
            for i, t in enumerate(turns)
        ]
    
    def _quick_embed(self, text: str) -> np.ndarray:
        """Fast hash-based embedding."""
        words = list(set(re.findall(r'\b[a-zA-Z]+\b', text.lower())))[:20]
        return np.array([hash(w) % 100 / 100.0 for w in words] + [0.0] * (20 - len(words)))
    
    def _bm25_score(self, query: str, text: str) -> float:
        """Compute BM25-like score."""
        query_terms = set(re.findall(r'\b[a-zA-Z]+\b', query.lower()))
        text_terms = set(re.findall(r'\b[a-zA-Z]+\b', text.lower()))
        overlap = len(query_terms & text_terms)
        return overlap / (len(query_terms) + 0.1)
    
    def _vector_score(self, query_emb: np.ndarray, cand_emb: np.ndarray) -> float:
        """Cosine similarity."""
        return np.dot(query_emb, cand_emb) / (np.linalg.norm(query_emb) * np.linalg.norm(cand_emb) + 1e-8)
    
    def _cross_encoder_score(self, query: str, text: str) -> float:
        """
        Simulated cross-encoder score.
        In practice, this would use a real cross-encoder model.
        """
        # Simple heuristic: overlap ratio + length penalty
        query_words = set(re.findall(r'\b[a-zA-Z]+\b', query.lower()))
        text_words = set(re.findall(r'\b[a-zA-Z]+\b', text.lower()))
        
        if not query_words:
            return 0
        
        overlap = len(query_words & text_words)
        precision = overlap / len(query_words)
        
        # Length penalty (prefer medium-length contexts)
        text_len = len(text.split())
        length_penalty = 1.0 if 10 <= text_len <= 100 else 0.8
        
        return precision * length_penalty
    
    def answer(self, question) -> Dict[str, Any]:
        """Three-stage cascade retrieval."""
        start_time = time.time()
        
        # Stage 1: BM25 for fast candidate selection
        stage1_scores = []
        for cand in self.candidates:
            score = self._bm25_score(question.question, cand['text'])
            stage1_scores.append((cand['index'], score))
        
        stage1_top = sorted(stage1_scores, key=lambda x: -x[1])[:self.stage1_top_k]
        stage1_ids = {idx for idx, _ in stage1_top}
        
        # Stage 2: Vector re-ranking
        query_emb = self._quick_embed(question.question)
        stage2_scores = []
        for idx in stage1_ids:
            cand = self.candidates[idx]
            score = self._vector_score(query_emb, cand['embedding'])
            stage2_scores.append((idx, score))
        
        stage2_top = sorted(stage2_scores, key=lambda x: -x[1])[:self.stage2_top_k]
        stage2_ids = {idx for idx, _ in stage2_top}
        
        # Stage 3: Cross-encoder final ranking
        stage3_scores = []
        for idx in stage2_ids:
            cand = self.candidates[idx]
            score = self._cross_encoder_score(question.question, cand['text'])
            stage3_scores.append((idx, score))
        
        stage3_top = sorted(stage3_scores, key=lambda x: -x[1])[:self.final_top_k]
        
        # Build context
        context_parts = []
        total_tokens = 0
        for idx, score in stage3_top:
            text = self.candidates[idx]['text']
            context_parts.append(text)
            total_tokens += len(text) // 4
        
        context = "\n".join(context_parts)
        latency = time.time() - start_time
        
        result = {
            'answer': "",
            'context': context,
            'tokens': total_tokens,
            'latency': latency,
            'stages': {
                'stage1': len(stage1_ids),
                'stage2': len(stage2_ids),
                'stage3': len(stage3_top)
            }
        }
        
        self.metrics['total_queries'] += 1
        self.metrics['total_time'] += latency
        self.metrics['tokens_retrieved'] += total_tokens
        
        return result


if __name__ == "__main__":
    from locomo_loader import load_locomo
    
    loader = load_locomo()
    conv = loader.conversations[0]
    
    print("Testing PageAwareRAGStrategy...")
    strategy = PageAwareRAGStrategy(page_size=512, top_k=3)
    strategy.setup(conv)
    print(f"Built {len(strategy.pages)} pages")
    
    if conv.questions:
        q = conv.questions[0]
        result = strategy.answer(q)
        print(f"Question: {q.question}")
        print(f"Retrieved {result.get('pages_retrieved', 0)} pages")
        print(f"Tokens: {result['tokens']}")
        print(f"Latency: {result['latency']*1000:.2f}ms")
