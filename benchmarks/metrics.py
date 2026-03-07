"""
Evaluation metrics for LOCOMO benchmark.
Adapted from LOCOMO's evaluation.py
"""
import re
import string
import unicodedata
from typing import List, Dict
from collections import Counter
import numpy as np

try:
    from nltk.stem import PorterStemmer
    ps = PorterStemmer()
except ImportError:
    # Fallback if nltk not available
    class FakeStemmer:
        def stem(self, word):
            return word
    ps = FakeStemmer()


class SimpleTokenizer:
    """Tokenizer for matching answers."""
    ALPHA_NUM = r'[\p{L}\p{N}\p{M}]+'
    NON_WS = r'[^\p{Z}\p{C}]'
    
    def __init__(self):
        import regex
        self._regexp = regex.compile(
            '(%s)|(%s)' % (self.ALPHA_NUM, self.NON_WS),
            flags=regex.IGNORECASE + regex.UNICODE + regex.MULTILINE
        )
    
    def tokenize(self, text: str, uncased: bool = False) -> List[str]:
        matches = [m for m in self._regexp.finditer(text)]
        if uncased:
            tokens = [m.group().lower() for m in matches]
        else:
            tokens = [m.group() for m in matches]
        return tokens


def normalize_answer(s: str) -> str:
    """Normalize answer string for comparison."""
    s = s.replace(',', "")
    
    def remove_articles(text):
        return re.sub(r'\b(a|an|the|and)\b', ' ', text)
    
    def white_space_fix(text):
        return ' '.join(text.split())
    
    def remove_punc(text):
        exclude = set(string.punctuation)
        return ''.join(ch for ch in text if ch not in exclude)
    
    def lower(text):
        return text.lower()
    
    return white_space_fix(remove_articles(remove_punc(lower(s))))


def f1_score(prediction: str, ground_truth: str) -> float:
    """Compute token-level F1 score with stemming."""
    prediction_tokens = [ps.stem(w) for w in normalize_answer(prediction).split()]
    ground_truth_tokens = [ps.stem(w) for w in normalize_answer(ground_truth).split()]
    
    common = Counter(prediction_tokens) & Counter(ground_truth_tokens)
    num_same = sum(common.values())
    
    if num_same == 0:
        return 0.0
    
    precision = 1.0 * num_same / len(prediction_tokens)
    recall = 1.0 * num_same / len(ground_truth_tokens)
    f1 = (2 * precision * recall) / (precision + recall)
    
    return f1


def f1_multi_answer(prediction: str, ground_truth: str) -> float:
    """
    F1 for multi-answer questions (comma-separated).
    Used for multi-hop questions.
    """
    predictions = [p.strip() for p in prediction.split(',')]
    ground_truths = [g.strip() for g in ground_truth.split(',')]
    
    scores = []
    for gt in ground_truths:
        max_f1 = max([f1_score(pred, gt) for pred in predictions])
        scores.append(max_f1)
    
    return np.mean(scores)


def exact_match_score(prediction: str, ground_truth: str) -> bool:
    """Check if prediction exactly matches ground truth."""
    pred = normalize_answer(prediction)
    gt = normalize_answer(ground_truth)
    return set(pred.split()) == set(gt.split())


def bleu1_score(prediction: str, ground_truth: str) -> float:
    """
    Compute BLEU-1 score (unigram precision with brevity penalty).
    Simplified implementation.
    """
    pred_tokens = normalize_answer(prediction).split()
    gt_tokens = normalize_answer(ground_truth).split()
    
    if len(pred_tokens) == 0:
        return 0.0
    
    # Unigram precision
    pred_counts = Counter(pred_tokens)
    gt_counts = Counter(gt_tokens)
    
    clipped_counts = sum((pred_counts & gt_counts).values())
    precision = clipped_counts / len(pred_tokens)
    
    # Brevity penalty
    bp = 1.0
    if len(pred_tokens) < len(gt_tokens):
        bp = np.exp(1 - len(gt_tokens) / len(pred_tokens))
    
    return bp * precision


def evaluate_question(prediction: str, question) -> Dict[str, float]:
    """
    Evaluate a single question based on its category.
    
    Args:
        prediction: Model's predicted answer
        question: Question object with answer and category
        
    Returns:
        Dictionary with F1 and BLEU-1 scores
    """
    answer = str(question.answer)
    
    # Handle temporal questions (category 3) - may have date format variations
    if question.category == 3:
        # Try different date formats
        answer = answer.split(';')[0].strip()
    
    # Compute scores based on category
    if question.category == 1:  # Multi-hop
        f1 = f1_multi_answer(prediction, answer)
    elif question.category == 5:  # Adversarial
        # Check if model correctly identifies as unanswerable
        pred_lower = prediction.lower()
        if 'no information' in pred_lower or 'not mentioned' in pred_lower:
            f1 = 1.0
        else:
            f1 = 0.0
    else:  # Single-hop, temporal, open-domain
        f1 = f1_score(prediction, answer)
    
    bleu = bleu1_score(prediction, answer)
    
    return {
        'f1': f1,
        'bleu': bleu,
        'exact_match': exact_match_score(prediction, answer)
    }


def aggregate_metrics(results: List[Dict]) -> Dict[str, float]:
    """Aggregate metrics across multiple questions."""
    if not results:
        return {'f1': 0.0, 'bleu': 0.0, 'exact_match': 0.0}
    
    return {
        'f1': np.mean([r['f1'] for r in results]),
        'bleu': np.mean([r['bleu'] for r in results]),
        'exact_match': np.mean([r['exact_match'] for r in results])
    }


class LLMJudge:
    """
    LLM-as-a-Judge evaluation.
    Uses OpenAI API to evaluate answer correctness.
    """
    
    JUDGE_PROMPT = """Your task is to label an answer to a question as "CORRECT" or "WRONG".
You will be given the following data:
    (1) a question (posed by one user to another user),
    (2) a 'gold' (ground truth) answer,
    (3) a generated answer
which you will score as CORRECT/WRONG.

The point of the question is to ask about something one user should know about the other user based on their prior conversations.
The gold answer will usually be a concise and short answer that includes the referenced topic.

The generated answer might be much longer, but you should be generous with your grading - as long as it touches on the same topic as the gold answer, it should be counted as CORRECT.

For time related questions, the gold answer will be a specific date, month, year, etc. The generated answer might be much longer or use relative time references (like 'last Tuesday' or 'next month'), but you should be generous with your grading - as long as it refers to the same date or time period as the gold answer, it should be counted as CORRECT.

Now it's time for the real question:

Question: {question}

Gold answer: {gold_answer}

Generated answer: {generated_answer}

First, provide a short (one sentence) explanation of your reasoning, then finish with CORRECT or WRONG.
Do NOT include both CORRECT and WRONG in your response.

Just return the label CORRECT or WRONG in a json format with the key as "label".
"""
    
    def __init__(self, model: str = "gpt-4o-mini"):
        self.model = model
        self.client = None
        try:
            import openai
            self.client = openai.OpenAI()
        except:
            pass
    
    def evaluate(self, question: str, gold_answer: str, generated_answer: str) -> Dict:
        """Evaluate a single answer using LLM judge."""
        if not self.client:
            return {'score': 50.0, 'label': 'UNKNOWN', 'explanation': 'OpenAI not available'}
        
        prompt = self.JUDGE_PROMPT.format(
            question=question,
            gold_answer=gold_answer,
            generated_answer=generated_answer
        )
        
        try:
            response = self.client.chat.completions.create(
                model=self.model,
                messages=[{"role": "user", "content": prompt}],
                temperature=0.0,
                max_tokens=150
            )
            
            content = response.choices[0].message.content
            
            # Parse result
            label = "WRONG"
            if "CORRECT" in content.upper():
                label = "CORRECT"
            
            score = 100.0 if label == "CORRECT" else 0.0
            
            return {
                'score': score,
                'label': label,
                'explanation': content
            }
        except Exception as e:
            return {'score': 50.0, 'label': 'ERROR', 'explanation': str(e)}


if __name__ == "__main__":
    # Test metrics
    pred = "Alice moved to San Francisco in 2022"
    gt = "Alice moved to San Francisco in 2022"
    
    print(f"Prediction: {pred}")
    print(f"Ground Truth: {gt}")
    print(f"F1: {f1_score(pred, gt):.3f}")
    print(f"BLEU-1: {bleu1_score(pred, gt):.3f}")
    print(f"Exact Match: {exact_match_score(pred, gt)}")
    
    # Test with variation
    pred2 = "Alice relocated to SF last year"
    print(f"\nPrediction: {pred2}")
    print(f"Ground Truth: {gt}")
    print(f"F1: {f1_score(pred2, gt):.3f}")
    print(f"BLEU-1: {bleu1_score(pred2, gt):.3f}")
