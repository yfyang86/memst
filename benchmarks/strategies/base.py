"""
Base strategy interface for LOCOMO benchmark.
"""
from abc import ABC, abstractmethod
from typing import List, Dict, Any
import time


class BenchmarkStrategy(ABC):
    """Base class for benchmark strategies."""
    
    def __init__(self, name: str):
        self.name = name
        self.metrics = {
            'total_queries': 0,
            'total_time': 0.0,
            'tokens_retrieved': 0,
        }
    
    @abstractmethod
    def setup(self, conversation) -> None:
        """
        Setup the strategy with a conversation.
        This includes indexing, memory construction, etc.
        """
        pass
    
    @abstractmethod
    def answer(self, question) -> Dict[str, Any]:
        """
        Answer a question.
        
        Returns:
            Dictionary with:
                - answer: The predicted answer string
                - context: Retrieved context (for analysis)
                - tokens: Number of tokens used
                - latency: Time taken (seconds)
        """
        pass
    
    def reset_metrics(self):
        """Reset metrics."""
        self.metrics = {
            'total_queries': 0,
            'total_time': 0.0,
            'tokens_retrieved': 0,
        }
    
    def get_metrics(self) -> Dict[str, float]:
        """Get current metrics."""
        metrics = self.metrics.copy()
        if metrics['total_queries'] > 0:
            metrics['avg_latency'] = metrics['total_time'] / metrics['total_queries']
            metrics['avg_tokens'] = metrics['tokens_retrieved'] / metrics['total_queries']
        return metrics


class FullContextStrategy(BenchmarkStrategy):
    """Baseline: Use full conversation as context."""
    
    def __init__(self):
        super().__init__("Full-Context")
        self.conversation_text = ""
        self.token_count = 0
    
    def setup(self, conversation) -> None:
        """Store full conversation text."""
        self.conversation_text = conversation.get_full_text()
        # Rough token estimate: 1 token ~= 4 characters
        self.token_count = len(self.conversation_text) // 4
    
    def answer(self, question) -> Dict[str, Any]:
        """Return full context for answering."""
        start_time = time.time()
        
        # In full-context, we don't actually answer here
        # We just provide the context for the LLM to use
        result = {
            'answer': "",  # To be filled by LLM
            'context': self.conversation_text,
            'tokens': self.token_count,
            'latency': time.time() - start_time
        }
        
        self.metrics['total_queries'] += 1
        self.metrics['total_time'] += result['latency']
        self.metrics['tokens_retrieved'] += result['tokens']
        
        return result


class MockLLM:
    """
    Mock LLM for testing strategies without API calls.
    Uses simple heuristics to extract answers from context.
    """
    
    def __init__(self):
        pass
    
    def generate(self, prompt: str, context: str, question: str) -> str:
        """
        Generate answer based on context.
        This is a simple heuristic baseline - not meant for real evaluation.
        """
        # Try to find answer in context
        # This is just a placeholder - real evaluation uses actual LLM
        
        # Simple keyword matching
        context_lower = context.lower()
        question_lower = question.lower()
        
        # Extract key terms from question (nouns, proper nouns)
        # This is a very naive implementation
        
        # For demonstration, return a placeholder
        return "[MOCK_ANSWER: " + question[:20] + "...]"


if __name__ == "__main__":
    # Test base strategy
    import sys
    sys.path.append('..')
    from locomo_loader import load_locomo
    
    loader = load_locomo()
    conv = loader.conversations[0]
    
    strategy = FullContextStrategy()
    strategy.setup(conv)
    
    print(f"Strategy: {strategy.name}")
    print(f"Token count: {strategy.token_count}")
    print(f"Setup complete for conversation: {conv.sample_id}")
