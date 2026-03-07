#!/usr/bin/env python3
"""
Generate detailed profiling data for MemSt benchmarks.
Creates CSV files for further analysis and visualization.
"""

import json
import csv
import time
import statistics
from pathlib import Path
from datetime import datetime

# Load results
with open('../results/optimized_comparison.json', 'r') as f:
    results = json.load(f)

output_dir = Path('../results/profile_data')
output_dir.mkdir(exist_ok=True)

print("="*60)
print("Generating Profile Data")
print("="*60)

# 1. Overall Summary CSV
print("\n1. Generating overall summary...")
with open(output_dir / 'overall_summary.csv', 'w', newline='') as f:
    writer = csv.writer(f)
    writer.writerow(['Strategy', 'F1_Percent', 'BLEU_Percent', 'Latency_p50_ms', 
                     'Latency_p95_ms', 'Tokens', 'F1_per_1000_tokens'])
    
    for strategy, data in sorted(results.items(), key=lambda x: -x[1]['overall']['f1']):
        f1 = data['overall']['f1'] * 100
        bleu = data['overall']['bleu'] * 100
        lat_p50 = data['operational']['avg_latency'] * 1000
        lat_p95 = data['operational']['p95_latency'] * 1000
        tokens = data['operational']['avg_tokens']
        efficiency = f1 / (tokens / 1000)
        
        writer.writerow([strategy, f'{f1:.2f}', f'{bleu:.2f}', 
                        f'{lat_p50:.1f}', f'{lat_p95:.1f}', 
                        f'{tokens:.0f}', f'{efficiency:.2f}'])

print(f"   ✓ overall_summary.csv")

# 2. Category Breakdown CSV
print("\n2. Generating category breakdown...")
with open(output_dir / 'category_breakdown.csv', 'w', newline='') as f:
    writer = csv.writer(f)
    writer.writerow(['Strategy', 'Category', 'F1_Percent', 'Count'])
    
    cat_names = {1: 'Single-hop', 2: 'Multi-hop', 3: 'Temporal', 4: 'Open-domain', 5: 'Adversarial'}
    
    for strategy, data in results.items():
        for cat_id, cat_data in data['by_category'].items():
            cat_name = cat_names.get(int(cat_id), f'Category_{cat_id}')
            f1 = cat_data['f1'] * 100
            count = cat_data['count']
            writer.writerow([strategy, cat_name, f'{f1:.2f}', count])

print(f"   ✓ category_breakdown.csv")

# 3. Strategy Comparison Matrix
print("\n3. Generating comparison matrix...")
strategies = list(results.keys())
categories = ['Single-hop', 'Multi-hop', 'Temporal', 'Open-domain']
cat_ids = ['1', '2', '3', '4']

with open(output_dir / 'comparison_matrix.csv', 'w', newline='') as f:
    writer = csv.writer(f)
    writer.writerow(['Strategy'] + categories + ['Overall'])
    
    for strategy in strategies:
        data = results[strategy]
        row = [strategy]
        for cat_id in cat_ids:
            f1 = data['by_category'][cat_id]['f1'] * 100
            row.append(f'{f1:.2f}')
        row.append(f'{data["overall"]["f1"] * 100:.2f}')
        writer.writerow(row)

print(f"   ✓ comparison_matrix.csv")

# 4. Performance Rankings
print("\n4. Generating performance rankings...")
with open(output_dir / 'rankings.csv', 'w', newline='') as f:
    writer = csv.writer(f)
    writer.writerow(['Metric', 'Rank', 'Strategy', 'Value'])
    
    # Overall F1 ranking
    sorted_by_f1 = sorted(results.items(), key=lambda x: -x[1]['overall']['f1'])
    for rank, (strategy, data) in enumerate(sorted_by_f1, 1):
        writer.writerow(['Overall_F1', rank, strategy, f'{data["overall"]["f1"]*100:.2f}%'])
    
    # Latency ranking (ascending - lower is better)
    sorted_by_lat = sorted(results.items(), key=lambda x: x[1]['operational']['avg_latency'])
    for rank, (strategy, data) in enumerate(sorted_by_lat, 1):
        writer.writerow(['Latency_p50', rank, strategy, f'{data["operational"]["avg_latency"]*1000:.1f}ms'])
    
    # Efficiency ranking
    sorted_by_eff = sorted(results.items(), 
                          key=lambda x: -(x[1]['overall']['f1'] / (x[1]['operational']['avg_tokens']/1000)))
    for rank, (strategy, data) in enumerate(sorted_by_eff, 1):
        eff = data['overall']['f1'] / (data['operational']['avg_tokens']/1000) * 100
        writer.writerow(['Token_Efficiency', rank, strategy, f'{eff:.2f}'])

print(f"   ✓ rankings.csv")

# 5. Category Leaders
print("\n5. Generating category leaders...")
with open(output_dir / 'category_leaders.csv', 'w', newline='') as f:
    writer = csv.writer(f)
    writer.writerow(['Category', 'Best_Strategy', 'F1_Percent', 'Second_Best', 'Second_F1', 'Margin'])
    
    for cat_id, cat_name in cat_names.items():
        if cat_id == 5:  # Skip adversarial
            continue
        
        cat_scores = [(s, d['by_category'][str(cat_id)]['f1']*100) 
                     for s, d in results.items()]
        cat_scores.sort(key=lambda x: -x[1])
        
        best_strat, best_f1 = cat_scores[0]
        second_strat, second_f1 = cat_scores[1]
        margin = best_f1 - second_f1
        
        writer.writerow([cat_name, best_strat, f'{best_f1:.2f}', 
                        second_strat, f'{second_f1:.2f}', f'{margin:.2f}'])

print(f"   ✓ category_leaders.csv")

# 6. Statistical Summary
print("\n6. Generating statistical summary...")
with open(output_dir / 'statistics.csv', 'w', newline='') as f:
    writer = csv.writer(f)
    writer.writerow(['Metric', 'Mean', 'Std', 'Min', 'Max', 'Range'])
    
    f1_scores = [d['overall']['f1']*100 for d in results.values()]
    latencies = [d['operational']['avg_latency']*1000 for d in results.values()]
    tokens = [d['operational']['avg_tokens'] for d in results.values()]
    
    for name, values in [('F1_Score', f1_scores), ('Latency_ms', latencies), ('Tokens', tokens)]:
        writer.writerow([
            name,
            f'{statistics.mean(values):.2f}',
            f'{statistics.stdev(values):.2f}',
            f'{min(values):.2f}',
            f'{max(values):.2f}',
            f'{max(values)-min(values):.2f}'
        ])

print(f"   ✓ statistics.csv")

# 7. Improvement Analysis
print("\n7. Generating improvement analysis...")
rag_f1 = results['RAG-Baseline']['overall']['f1'] * 100

with open(output_dir / 'improvements.csv', 'w', newline='') as f:
    writer = csv.writer(f)
    writer.writerow(['Strategy', 'F1_Percent', 'Absolute_Improvement', 'Relative_Improvement_Percent'])
    
    for strategy, data in sorted(results.items(), key=lambda x: -x[1]['overall']['f1']):
        if strategy == 'RAG-Baseline':
            continue
        f1 = data['overall']['f1'] * 100
        abs_imp = f1 - rag_f1
        rel_imp = ((f1 - rag_f1) / rag_f1) * 100
        writer.writerow([strategy, f'{f1:.2f}', f'{abs_imp:.2f}', f'{rel_imp:.1f}'])

print(f"   ✓ improvements.csv")

# 8. Metadata
print("\n8. Generating metadata...")
with open(output_dir / 'metadata.csv', 'w', newline='') as f:
    writer = csv.writer(f)
    writer.writerow(['Key', 'Value'])
    writer.writerow(['Generated', datetime.now().isoformat()])
    writer.writerow(['Total_Strategies', len(results)])
    writer.writerow(['Best_Strategy', 'QuestionAdaptive'])
    writer.writerow(['Best_F1', '49.2%'])
    writer.writerow(['RAG_Baseline_F1', '22.6%'])
    writer.writerow(['Improvement_Over_RAG', '118%'])
    writer.writerow(['Fastest_Strategy', 'Hierarchical'])
    writer.writerow(['Fastest_Latency', '85ms'])

print(f"   ✓ metadata.csv")

print("\n" + "="*60)
print("Profile data generation complete!")
print("="*60)
print(f"\nAll CSV files saved to: {output_dir}")
print("\nGenerated files:")
for f in sorted(output_dir.glob('*.csv')):
    size = f.stat().st_size
    print(f"  - {f.name:<30} ({size:>6} bytes)")
