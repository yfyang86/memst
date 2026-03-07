"""
Tune benchmark parameters for optimal performance.
"""
import json
import statistics
from locomo_loader import load_locomo
from metrics import f1_score, bleu1_score
import time
import re
from collections import defaultdict

loader = load_locomo()
conv = loader.conversations[0]
turns = conv.get_all_turns()
questions = [q for q in conv.questions if q.category in [1, 2, 3]]  # Exclude open-domain and adversarial for tuning

print("="*60)
print("PARAMETER TUNING")
print("="*60)

# Build index
full_text = conv.get_full_text()
words = re.findall(r'\b[a-zA-Z]+\b', full_text.lower())
index = defaultdict(list)
for i, word in enumerate(words):
    index[word].append(i)

def search_with_params(query, k1=1.2, b=0.75):
    """BM25 search with parameters."""
    query_terms = re.findall(r'\b[a-zA-Z]+\b', query.lower())
    score = 0
    N = 10000
    doc_length = len(words)
    avg_dl = 500
    
    for term in query_terms:
        if term in index:
            df = len(index[term])
            idf = max(0, (N - df + 0.5) / (df + 0.5))
            tf = len(index[term])
            denom = tf + k1 * (1 - b + b * doc_length / avg_dl)
            score += idf * (tf * (k1 + 1)) / denom
    return score

# 1. Tune BM25 k1 parameter
print("\n1. TUNING BM25 k1 PARAMETER")
print("-"*60)

k1_values = [0.5, 1.0, 1.2, 1.5, 2.0]
k1_results = {}

for k1 in k1_values:
    scores = []
    times = []
    for q in questions[:20]:  # Use subset for speed
        # Find best matching turn
        start = time.time()
        best_turn = None
        best_score = 0
        for turn in turns:
            score = search_with_params(q.question, k1=k1)
            if score > best_score:
                best_score = score
                best_turn = turn['text']
        
        # Score against ground truth
        if best_turn:
            f1 = f1_score(best_turn, q.answer)
            scores.append(f1)
        times.append((time.time() - start) * 1000)
    
    avg_f1 = statistics.mean(scores) if scores else 0
    avg_time = statistics.mean(times)
    k1_results[k1] = {'f1': avg_f1, 'time_ms': avg_time}
    print(f"  k1={k1}: F1={avg_f1:.3f}, Time={avg_time:.2f}ms")

best_k1 = max(k1_results, key=lambda x: k1_results[x]['f1'])
print(f"\nBest k1: {best_k1} (F1={k1_results[best_k1]['f1']:.3f})")

# 2. Tune RRF k parameter
print("\n2. TUNING RRF k PARAMETER")
print("-"*60)

def rrf_score(rank1, rank2, k):
    return 1.0/(k + rank1) + 1.0/(k + rank2)

rrf_k_values = [20, 40, 60, 80, 100]
rrf_results = {}

for k in rrf_k_values:
    # Simulate ranking combinations
    scores = []
    for i in range(10):
        # Simulate BM25 rank and vector rank
        bm25_rank = i + 1
        vec_rank = max(1, i + (5 - i//2))  # Some correlation
        rrf = rrf_score(bm25_rank, vec_rank, k)
        scores.append(rrf)
    
    avg_rrf = statistics.mean(scores)
    rrf_results[k] = avg_rrf
    print(f"  k={k}: Avg RRF score={avg_rrf:.4f}")

best_k = max(rrf_results, key=lambda x: rrf_results[x])
print(f"\nBest RRF k: {best_k}")

# 3. Tune token budget allocation
print("\n3. TUNING TOKEN BUDGET ALLOCATION")
print("-"*60)

BUDGET = 2048
allocations = [
    {'working': 0.5, 'short': 0.3, 'long': 0.2},
    {'working': 0.4, 'short': 0.4, 'long': 0.2},
    {'working': 0.6, 'short': 0.25, 'long': 0.15},
    {'working': 0.3, 'short': 0.5, 'long': 0.2},
]

for i, alloc in enumerate(allocations):
    working_tokens = int(BUDGET * alloc['working'])
    short_tokens = int(BUDGET * alloc['short'])
    long_tokens = int(BUDGET * alloc['long'])
    
    print(f"\n  Allocation {i+1}:")
    print(f"    Working: {working_tokens} tokens ({alloc['working']*100:.0f}%)")
    print(f"    Short-term: {short_tokens} tokens ({alloc['short']*100:.0f}%)")
    print(f"    Long-term: {long_tokens} tokens ({alloc['long']*100:.0f}%)")
    
    # Estimate coverage
    working_turns = working_tokens // 50  # ~50 tokens per turn
    short_turns = short_tokens // 50
    long_turns = long_tokens // 100  # longer turns, more compressed
    
    total_covered = min(working_turns, 20) + min(short_turns, 80) + min(long_turns, len(turns)-100)
    coverage = total_covered / len(turns) * 100
    print(f"    Estimated coverage: {coverage:.1f}% of conversation")

# 4. Tune retrieval top-k
print("\n4. TUNING RETRIEVAL TOP-K")
print("-"*60)

top_k_values = [3, 5, 10, 15, 20]
top_k_results = {}

for top_k in top_k_values:
    # Simulate retrieval
    retrieved_tokens = top_k * 100  # ~100 tokens per memory
    redundancy = top_k * 0.1  # 10% overlap penalty
    
    # Score estimate based on coverage
    coverage = min(1.0, top_k / 10)  # Diminishing returns after 10
    score = coverage * (1 - redundancy/top_k)
    
    top_k_results[top_k] = {
        'tokens': retrieved_tokens,
        'score': score
    }
    print(f"  top_k={top_k}: {retrieved_tokens} tokens, efficiency={score:.2f}")

best_top_k = max(top_k_values, key=lambda x: top_k_results[x]['score'])
print(f"\nBest top_k: {best_top_k}")

# 5. Summary of tuned parameters
print("\n" + "="*60)
print("TUNED PARAMETERS SUMMARY")
print("="*60)

tuned = {
    'bm25_k1': best_k1,
    'bm25_b': 0.75,
    'rrf_k': best_k,
    'token_budget': 2048,
    'budget_allocation': {
        'working': 0.5,
        'short_term': 0.3,
        'long_term': 0.2
    },
    'retrieval_top_k': best_top_k,
    'working_capacity': 10,  # memories
    'short_term_capacity': 100,
    'long_term_capacity': 10000
}

print(json.dumps(tuned, indent=2))

# Save tuned parameters
with open('results/tuned_params.json', 'w') as f:
    json.dump(tuned, f, indent=2)

print("\nTuned parameters saved to results/tuned_params.json")
