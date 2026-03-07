"""
Main evaluation runner for LOCOMO benchmark.
"""
import json
import argparse
import time
from pathlib import Path
from typing import List, Dict
import sys

# Add parent directory to path
sys.path.insert(0, str(Path(__file__).parent))

from locomo_loader import load_locomo, Question
from metrics import evaluate_question, aggregate_metrics, LLMJudge
from strategies.base import FullContextStrategy
from strategies.memst_strategy import MemStStrategy, RAGStrategy


class LoCoMoEvaluator:
    """Evaluator for LOCOMO benchmark."""
    
    def __init__(
        self,
        llm_client=None,
        use_llm_judge: bool = False,
        judge_model: str = "gpt-4o-mini"
    ):
        self.llm_client = llm_client
        self.use_llm_judge = use_llm_judge
        self.judge = LLMJudge(judge_model) if use_llm_judge else None
        
        self.results = []
    
    def answer_with_llm(self, question: str, context: str) -> str:
        """
        Generate answer using LLM.
        This uses the context retrieved by the strategy.
        """
        if not self.llm_client:
            # Fallback: extract sentences containing keywords
            # This is a naive baseline for testing
            words = question.lower().split()
            sentences = context.split('.')
            for sent in sentences:
                if any(w in sent.lower() for w in words if len(w) > 3):
                    return sent.strip()[:100]
            return "I don't have enough information to answer this question."
        
        # Use actual LLM
        prompt = f"""Based on the following conversation context, answer the question.
Be concise and specific.

Context:
{context[:4000]}

Question: {question}

Answer:"""
        
        try:
            # This would use the actual LLM client
            # For now, return a placeholder
            return "[LLM_ANSWER_PLACEHOLDER]"
        except Exception as e:
            return f"Error: {str(e)}"
    
    def evaluate_strategy(
        self,
        strategy,
        conversations,
        categories: List[int] = None
    ) -> Dict:
        """
        Evaluate a strategy on LOCOMO dataset.
        
        Args:
            strategy: Strategy instance
            conversations: List of conversations
            categories: Filter to specific question categories (optional)
        
        Returns:
            Results dictionary with metrics
        """
        all_results = []
        category_results = {1: [], 2: [], 3: [], 4: [], 5: []}
        
        for conv in conversations:
            print(f"\nEvaluating conversation: {conv.sample_id}")
            
            # Setup strategy
            setup_start = time.time()
            strategy.setup(conv)
            setup_time = time.time() - setup_start
            
            # Evaluate each question
            for q in conv.questions:
                # Filter by category if specified
                if categories and q.category not in categories:
                    continue
                
                # Get retrieval result from strategy
                result = strategy.answer(q)
                
                # Generate answer using LLM with context
                prediction = self.answer_with_llm(q.question, result['context'])
                result['answer'] = prediction
                
                # Compute metrics
                metrics = evaluate_question(prediction, q)
                
                # LLM-as-a-Judge
                judge_score = None
                if self.judge:
                    judge_result = self.judge.evaluate(
                        q.question, q.answer, prediction
                    )
                    judge_score = judge_result['score']
                
                # Store result
                eval_result = {
                    'conversation_id': conv.sample_id,
                    'question': q.question,
                    'ground_truth': q.answer,
                    'prediction': prediction,
                    'category': q.category,
                    'category_name': q.category_name,
                    'f1': metrics['f1'],
                    'bleu': metrics['bleu'],
                    'exact_match': metrics['exact_match'],
                    'judge_score': judge_score,
                    'tokens': result['tokens'],
                    'latency': result['latency']
                }
                
                all_results.append(eval_result)
                category_results[q.category].append(eval_result)
            
            # Cleanup
            if hasattr(strategy, 'cleanup'):
                strategy.cleanup()
        
        # Aggregate metrics
        summary = {
            'strategy': strategy.name,
            'total_questions': len(all_results),
            'setup_time': setup_time,
            'overall': aggregate_metrics(all_results),
            'by_category': {}
        }
        
        for cat_id, cat_results in category_results.items():
            if cat_results:
                summary['by_category'][cat_id] = {
                    'name': cat_results[0]['category_name'],
                    'count': len(cat_results),
                    'metrics': aggregate_metrics(cat_results)
                }
                if self.judge:
                    judge_scores = [r['judge_score'] for r in cat_results if r['judge_score'] is not None]
                    if judge_scores:
                        summary['by_category'][cat_id]['judge_mean'] = sum(judge_scores) / len(judge_scores)
        
        # Add operational metrics
        strategy_metrics = strategy.get_metrics()
        summary['operational'] = strategy_metrics
        
        self.results = all_results
        return summary
    
    def save_results(self, output_path: str):
        """Save detailed results to JSONL."""
        with open(output_path, 'w') as f:
            for result in self.results:
                f.write(json.dumps(result) + '\n')
        print(f"Results saved to {output_path}")


def print_summary(summary: Dict):
    """Print formatted summary."""
    print("\n" + "="*60)
    print(f"EVALUATION SUMMARY: {summary['strategy']}")
    print("="*60)
    
    print(f"\nTotal Questions: {summary['total_questions']}")
    print(f"Setup Time: {summary['setup_time']:.2f}s")
    
    print("\n--- Overall Metrics ---")
    metrics = summary['overall']
    print(f"F1 Score: {metrics['f1']:.3f}")
    print(f"BLEU-1: {metrics['bleu']:.3f}")
    print(f"Exact Match: {metrics['exact_match']:.3f}")
    
    print("\n--- By Category ---")
    for cat_id, cat_data in sorted(summary['by_category'].items()):
        print(f"\n{cat_data['name']} (n={cat_data['count']}):")
        m = cat_data['metrics']
        print(f"  F1: {m['f1']:.3f} | BLEU: {m['bleu']:.3f} | EM: {m['exact_match']:.3f}")
        if 'judge_mean' in cat_data:
            print(f"  Judge: {cat_data['judge_mean']:.1f}")
    
    print("\n--- Operational Metrics ---")
    op = summary['operational']
    print(f"Total Queries: {op['total_queries']}")
    print(f"Avg Latency: {op.get('avg_latency', 0):.3f}s")
    print(f"Avg Tokens: {op.get('avg_tokens', 0):.0f}")
    print("="*60)


def main():
    parser = argparse.ArgumentParser(description='Run LOCOMO benchmark')
    parser.add_argument('--strategy', choices=['full-context', 'memst', 'memst-kg', 'rag'],
                       default='memst', help='Evaluation strategy')
    parser.add_argument('--categories', nargs='+', type=int,
                       help='Filter to specific categories (1-5)')
    parser.add_argument('--output', type=str, default='results/results.jsonl',
                       help='Output file for detailed results')
    parser.add_argument('--use-llm-judge', action='store_true',
                       help='Use LLM-as-a-Judge evaluation')
    parser.add_argument('--limit-conversations', type=int,
                       help='Limit number of conversations for testing')
    
    args = parser.parse_args()
    
    # Load dataset
    print("Loading LOCOMO dataset...")
    loader = load_locomo()
    stats = loader.get_statistics()
    print(f"Loaded {stats['num_conversations']} conversations, {stats['total_questions']} questions")
    
    # Select conversations
    conversations = loader.conversations
    if args.limit_conversations:
        conversations = conversations[:args.limit_conversations]
    
    # Create strategy
    print(f"\nInitializing strategy: {args.strategy}")
    if args.strategy == 'full-context':
        strategy = FullContextStrategy()
    elif args.strategy == 'memst':
        strategy = MemStStrategy(use_kg=False)
    elif args.strategy == 'memst-kg':
        strategy = MemStStrategy(use_kg=True)
    elif args.strategy == 'rag':
        strategy = RAGStrategy(chunk_size=512, top_k=2)
    else:
        raise ValueError(f"Unknown strategy: {args.strategy}")
    
    # Create evaluator
    evaluator = LoCoMoEvaluator(
        use_llm_judge=args.use_llm_judge
    )
    
    # Run evaluation
    print("\nRunning evaluation...")
    start_time = time.time()
    summary = evaluator.evaluate_strategy(strategy, conversations, args.categories)
    eval_time = time.time() - start_time
    
    print(f"\nEvaluation completed in {eval_time:.2f}s")
    
    # Print summary
    print_summary(summary)
    
    # Save results
    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    evaluator.save_results(output_path)
    
    # Save summary
    summary_path = output_path.with_suffix('.summary.json')
    with open(summary_path, 'w') as f:
        json.dump(summary, f, indent=2)
    print(f"Summary saved to {summary_path}")


if __name__ == "__main__":
    main()
