"""
Optimized MemSt strategies with advanced features.
"""
import sys
sys.path.insert(0, str(__file__).rsplit('/', 2)[0])

import re
import json
import time
from typing import Dict, Any, List, Optional, Tuple
from collections import defaultdict
from dataclasses import dataclass
import numpy as np

try:
    from strategies.base import BenchmarkStrategy
except ImportError:
    from base import BenchmarkStrategy


@dataclass
class TemporalEvent:
    """Temporal event extracted from conversation."""
    entity: str
    event: str
    time_expr: str
    normalized_time: str
    session: str
    text: str


@dataclass
class EntityMention:
    """Entity mention in conversation."""
    name: str
    entity_type: str
    session: str
    mentions: List[str]


class TemporalKGStrategy(BenchmarkStrategy):
    """
    Strategy optimized for temporal questions.
    Extracts and indexes temporal patterns.
    """
    
    def __init__(
        self,
        total_budget: int = 2048,
        timeline_budget: float = 0.4,
        context_budget: float = 0.6
    ):
        super().__init__("TemporalKG")
        self.total_budget = total_budget
        self.timeline_budget = timeline_budget
        self.context_budget = context_budget
        self.temporal_events: List[TemporalEvent] = []
        self.entity_timeline: Dict[str, List[TemporalEvent]] = defaultdict(list)
        self.session_dates: Dict[str, str] = {}
    
    def _extract_time_expressions(self, text: str) -> List[Tuple[str, str]]:
        """Extract time expressions from text."""
        patterns = [
            # Dates
            (r'\b(January|February|March|April|May|June|July|August|September|October|November|December|Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)[\s,]+\d{1,2}(?:[\s,]+\d{4})?\b', 'date'),
            # Years
            (r'\b(in\s+)?(19|20)\d{2}\b', 'year'),
            # Relative time
            (r'\b(last|next|this)\s+(week|month|year|Monday|Tuesday|Wednesday|Thursday|Friday|Saturday|Sunday)\b', 'relative'),
            # Specific days
            (r'\b(Monday|Tuesday|Wednesday|Thursday|Friday|Saturday|Sunday)\b', 'day'),
        ]
        
        matches = []
        for pattern, ptype in patterns:
            for match in re.finditer(pattern, text, re.IGNORECASE):
                matches.append((match.group(), ptype))
        return matches
    
    def _extract_entities(self, text: str) -> List[str]:
        """Extract potential entities (capitalized phrases)."""
        # Match capitalized words and names
        entities = re.findall(r'\b[A-Z][a-z]+(?:\s+[A-Z][a-z]+)*\b', text)
        # Filter common non-entities
        stop_words = {'I', 'The', 'A', 'An', 'This', 'That', 'It', 'He', 'She', 'We', 'They'}
        return [e for e in entities if e not in stop_words][:5]
    
    def setup(self, conversation) -> None:
        """Build temporal KG from conversation."""
        turns = conversation.get_all_turns()
        
        # Store session dates
        for i, date in enumerate(conversation.session_dates):
            self.session_dates[f"session_{i+1}"] = date
        
        current_session = "session_1"
        
        for i, turn in enumerate(turns):
            text = turn['text']
            speaker = turn['speaker']
            
            # Update session
            session_num = i // 20 + 1
            current_session = f"session_{session_num}"
            
            # Extract temporal expressions
            time_exprs = self._extract_time_expressions(text)
            entities = self._extract_entities(text)
            
            for time_str, time_type in time_exprs:
                for entity in entities[:2]:  # Limit to top 2 entities
                    event = TemporalEvent(
                        entity=entity,
                        event=text[:100],  # Truncate
                        time_expr=time_str,
                        normalized_time=self._normalize_time(time_str, time_type, current_session),
                        session=current_session,
                        text=text
                    )
                    self.temporal_events.append(event)
                    self.entity_timeline[entity].append(event)
    
    def _normalize_time(self, time_str: str, time_type: str, session: str) -> str:
        """Normalize time expression to standard format."""
        if time_type == 'year':
            year_match = re.search(r'(19|20)\d{2}', time_str)
            return year_match.group() if year_match else time_str
        elif time_type == 'date':
            # Extract components
            return time_str  # Keep as-is for simplicity
        elif time_type == 'relative':
            # Could use session date as reference
            session_date = self.session_dates.get(session, "")
            return f"{time_str} (ref: {session_date})"
        return time_str
    
    def _get_timeline_context(self, entity: str) -> str:
        """Get timeline context for an entity."""
        events = self.entity_timeline.get(entity, [])
        if not events:
            return ""
        
        timeline = [f"- {e.time_expr}: {e.event[:50]}..." for e in events[:5]]
        return f"Timeline for {entity}:\n" + "\n".join(timeline)
    
    def answer(self, question) -> Dict[str, Any]:
        """Answer temporal question using KG."""
        start_time = time.time()
        
        # Extract entities from question
        q_entities = self._extract_entities(question.question)
        
        # Build context
        context_parts = []
        total_tokens = 0
        
        # Add timelines for relevant entities
        for entity in q_entities:
            timeline = self._get_timeline_context(entity)
            if timeline:
                tokens = len(timeline) // 4
                if total_tokens + tokens < self.total_budget * self.timeline_budget:
                    context_parts.append(timeline)
                    total_tokens += tokens
        
        # Add supporting temporal events
        for event in self.temporal_events[:20]:
            if any(e in question.question for e in [event.entity, event.time_expr]):
                text = f"[{event.time_expr}] {event.text[:80]}..."
                tokens = len(text) // 4
                if total_tokens + tokens < self.total_budget:
                    context_parts.append(text)
                    total_tokens += tokens
        
        context = "\n\n".join(context_parts)
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


class MultiHopKGStrategy(BenchmarkStrategy):
    """
    Strategy for multi-hop questions using entity graph traversal.
    """
    
    def __init__(
        self,
        max_hops: int = 2,
        budget_per_hop: int = 500
    ):
        super().__init__(f"MultiHopKG-{max_hops}")
        self.max_hops = max_hops
        self.budget_per_hop = budget_per_hop
        self.entity_relations: Dict[str, List[Dict]] = defaultdict(list)
        self.turns_by_entity: Dict[str, List[int]] = defaultdict(list)
    
    def _extract_relations(self, text: str) -> List[Tuple[str, str, str]]:
        """
        Extract entity relations from text.
        Returns (subject, relation, object) triples.
        """
        triples = []
        
        # Simple patterns for relation extraction
        patterns = [
            (r'([A-Z][a-z]+)\s+(?:is|was)\s+(?:a|an)?\s*([a-z]+)\s+(?:of|at|in)\s+([A-Z][a-z]+)', 1, 2, 3),
            (r'([A-Z][a-z]+)\s+(?:works?|worked)\s+(?:at|for)\s+([A-Z][a-z]+)', 1, 'works_at', 2),
            (r'([A-Z][a-z]+)\s+(?:moved|went)\s+to\s+([A-Z][a-z]+)', 1, 'moved_to', 2),
        ]
        
        for pattern, subj_idx, rel_idx, obj_idx in patterns:
            for match in re.finditer(pattern, text):
                groups = match.groups()
                if len(groups) >= 3:
                    subj = groups[subj_idx - 1]
                    rel = groups[rel_idx - 1] if isinstance(rel_idx, int) else rel_idx
                    obj = groups[obj_idx - 1]
                    triples.append((subj, rel, obj))
        
        return triples
    
    def setup(self, conversation) -> None:
        """Build entity relation graph."""
        turns = conversation.get_all_turns()
        
        for i, turn in enumerate(turns):
            text = turn['text']
            speaker = turn['speaker']
            
            # Extract entities
            entities = re.findall(r'\b[A-Z][a-z]+(?:\s+[A-Z][a-z]+)*\b', text)
            entities = [e for e in entities if e not in ['I', 'The', 'A', 'An', 'It']]
            
            # Index turn by entity
            for entity in entities:
                self.turns_by_entity[entity].append(i)
            
            # Extract relations
            triples = self._extract_relations(text)
            for subj, rel, obj in triples:
                relation = {
                    'relation': rel,
                    'object': obj,
                    'text': text,
                    'turn_index': i
                }
                self.entity_relations[subj].append(relation)
    
    def _traverse_graph(self, start_entity: str, max_depth: int = 2) -> List[Dict]:
        """Traverse entity graph from starting entity."""
        visited = set()
        results = []
        queue = [(start_entity, 0)]
        
        while queue and len(results) < 20:
            entity, depth = queue.pop(0)
            
            if entity in visited or depth > max_depth:
                continue
            
            visited.add(entity)
            
            # Get relations for this entity
            relations = self.entity_relations.get(entity, [])
            for rel in relations:
                results.append({
                    'entity': entity,
                    'relation': rel['relation'],
                    'object': rel['object'],
                    'text': rel['text'],
                    'depth': depth
                })
                
                # Add object to queue for next hop
                if depth < max_depth:
                    queue.append((rel['object'], depth + 1))
        
        return results
    
    def answer(self, question) -> Dict[str, Any]:
        """Answer multi-hop question using graph traversal."""
        start_time = time.time()
        
        # Extract starting entities from question
        q_entities = re.findall(r'\b[A-Z][a-z]+(?:\s+[A-Z][a-z]+)*\b', question.question)
        q_entities = [e for e in q_entities if e not in ['I', 'The', 'A', 'An']][:3]
        
        # Traverse graph for each entity
        all_paths = []
        for entity in q_entities:
            paths = self._traverse_graph(entity, self.max_hops)
            all_paths.extend(paths)
        
        # Build context
        context_parts = []
        total_tokens = 0
        seen_texts = set()
        
        # Add paths
        for path in all_paths[:10]:  # Limit to top 10
            text = f"[{path['entity']} --{path['relation']}--> {path['object']}]: {path['text'][:60]}..."
            if text not in seen_texts:
                tokens = len(text) // 4
                if total_tokens + tokens < self.budget_per_hop * self.max_hops:
                    context_parts.append(text)
                    seen_texts.add(text)
                    total_tokens += tokens
        
        # Add direct mentions
        for entity in q_entities:
            for turn_idx in self.turns_by_entity.get(entity, [])[:3]:
                text = f"[Mention of {entity}]: Turn {turn_idx}"
                if text not in seen_texts:
                    tokens = len(text) // 4
                    if total_tokens < self.budget_per_hop * self.max_hops:
                        context_parts.append(text)
                        seen_texts.add(text)
                        total_tokens += tokens
        
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


class QuestionAdaptiveStrategy(BenchmarkStrategy):
    """
    Adaptive strategy that selects retrieval method based on question type.
    """
    
    def __init__(self, total_budget: int = 2048):
        super().__init__("QuestionAdaptive")
        self.total_budget = total_budget
        
        # Sub-strategies
        self.temporal_strategy = None
        self.multihop_strategy = None
        self.standard_strategy = None
    
    def setup(self, conversation) -> None:
        """Setup all sub-strategies."""
        # Initialize sub-strategies
        self.temporal_strategy = TemporalKGStrategy(
            total_budget=self.total_budget
        )
        self.multihop_strategy = MultiHopKGStrategy(
            budget_per_hop=self.total_budget // 4
        )
        # Import here to avoid circular import
        try:
            from strategies.advanced_strategies import PageAwareRAGStrategy
        except ImportError:
            from advanced_strategies import PageAwareRAGStrategy
        self.standard_strategy = PageAwareRAGStrategy(
            page_size=512,
            top_k=5
        )
        
        # Setup each
        self.temporal_strategy.setup(conversation)
        self.multihop_strategy.setup(conversation)
        self.standard_strategy.setup(conversation)
    
    def _classify_question(self, question) -> str:
        """Classify question type for routing."""
        q_text = question.question.lower()
        
        # Temporal indicators
        temporal_words = ['when', 'what time', 'what year', 'what month', 'date', 'how long', 'how many years']
        if any(w in q_text for w in temporal_words):
            return 'temporal'
        
        # Multi-hop indicators
        multihop_words = ['friend', 'colleague', 'manager', 'sibling', 'parent', 'child', 'who']
        if any(w in q_text for w in multihop_words):
            return 'multihop'
        
        # Use question category if available
        if question.category == 3:
            return 'temporal'
        elif question.category == 2:
            return 'multihop'
        
        return 'standard'
    
    def answer(self, question) -> Dict[str, Any]:
        """Route to appropriate strategy."""
        q_type = self._classify_question(question)
        
        if q_type == 'temporal' and self.temporal_strategy:
            result = self.temporal_strategy.answer(question)
            result['strategy_used'] = 'temporal'
        elif q_type == 'multihop' and self.multihop_strategy:
            result = self.multihop_strategy.answer(question)
            result['strategy_used'] = 'multihop'
        else:
            result = self.standard_strategy.answer(question)
            result['strategy_used'] = 'standard'
        
        self.metrics['total_queries'] += 1
        self.metrics['total_time'] += result['latency']
        self.metrics['tokens_retrieved'] += result['tokens']
        
        return result


if __name__ == "__main__":
    from locomo_loader import load_locomo
    
    loader = load_locomo()
    conv = loader.conversations[0]
    
    print("Testing TemporalKGStrategy...")
    strategy = TemporalKGStrategy()
    strategy.setup(conv)
    print(f"Extracted {len(strategy.temporal_events)} temporal events")
    
    temporal_qs = [q for q in conv.questions if q.category == 3][:2]
    for q in temporal_qs:
        result = strategy.answer(q)
        print(f"\nQ: {q.question}")
        print(f"Tokens: {result['tokens']}, Latency: {result['latency']*1000:.1f}ms")
