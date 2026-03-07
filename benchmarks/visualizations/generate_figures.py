#!/usr/bin/env python3
"""
Generate benchmark visualization figures for MemSt paper.
Uses seaborn and matplotlib for publication-quality figures.
"""

import json
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns
from pathlib import Path

# Set style for publication
sns.set_style("whitegrid")
sns.set_context("paper", font_scale=1.3)
plt.rcParams['figure.dpi'] = 300
plt.rcParams['savefig.dpi'] = 300
plt.rcParams['font.family'] = 'sans-serif'
plt.rcParams['font.sans-serif'] = ['Arial', 'DejaVu Sans']

# Color palette
COLORS = {
    'question_adaptive': '#2E86AB',  # Blue
    'hierarchical': '#A23B72',       # Purple
    'cascade': '#F18F01',            # Orange
    'weighted_rrf': '#C73E1D',       # Red
    'multihop_kg': '#3B1F2B',        # Dark
    'temporal_kg': '#95C623',        # Green
    'pagerag': '#6A4C93',            # Violet
    'rag_baseline': '#8B8B8B',       # Gray
}

def load_results():
    """Load benchmark results."""
    with open('../results/optimized_comparison.json', 'r') as f:
        return json.load(f)

def prepare_data(results):
    """Convert results to pandas DataFrame."""
    rows = []
    for strategy, data in results.items():
        row = {
            'Strategy': strategy,
            'F1': data['overall']['f1'] * 100,
            'BLEU': data['overall']['bleu'] * 100,
            'Latency_p50': data['operational']['avg_latency'] * 1000,
            'Latency_p95': data['operational']['p95_latency'] * 1000,
            'Tokens': data['operational']['avg_tokens'],
        }
        # Add category scores
        for cat_id, cat_data in data['by_category'].items():
            cat_name = {1: 'Single', 2: 'Multi', 3: 'Temporal', 4: 'Open', 5: 'Adv'}[int(cat_id)]
            row[f'F1_{cat_name}'] = cat_data['f1'] * 100
        rows.append(row)
    return pd.DataFrame(rows)

def figure_1_overall_performance(df, output_dir):
    """Figure 1: Overall Performance Comparison (Main Figure)"""
    fig, ax = plt.subplots(figsize=(10, 6))
    
    # Sort by F1
    df_sorted = df.sort_values('F1', ascending=True)
    
    # Colors
    colors = [COLORS.get(s.lower().replace('-', '_').replace(' ', '_'), '#333333') 
              for s in df_sorted['Strategy']]
    
    bars = ax.barh(df_sorted['Strategy'], df_sorted['F1'], color=colors, edgecolor='black', linewidth=0.5)
    
    # Add value labels
    for i, (bar, val) in enumerate(zip(bars, df_sorted['F1'])):
        ax.text(val + 1, bar.get_y() + bar.get_height()/2, 
                f'{val:.1f}%', va='center', fontsize=10, fontweight='bold')
    
    ax.set_xlabel('F1 Score (%)', fontweight='bold', fontsize=12)
    ax.set_title('MemSt Overall Performance on LOCOMO Benchmark', fontweight='bold', fontsize=14, pad=20)
    ax.set_xlim(0, 55)
    ax.grid(axis='x', alpha=0.3)
    
    # Add improvement annotation
    rag_f1 = df[df['Strategy'] == 'RAG-Baseline']['F1'].values[0]
    ax.axvline(rag_f1, color='gray', linestyle='--', alpha=0.5, linewidth=2)
    ax.text(rag_f1 + 2, 0.5, f'RAG Baseline\n{rag_f1:.1f}%', 
            fontsize=9, color='gray', style='italic')
    
    plt.tight_layout()
    plt.savefig(output_dir / 'figure1_overall_performance.png', bbox_inches='tight')
    plt.savefig(output_dir / 'figure1_overall_performance.pdf', bbox_inches='tight')
    print("✓ Figure 1: Overall Performance")
    plt.close()

def figure_2_category_heatmap(df, output_dir):
    """Figure 2: Performance by Category (Heatmap)"""
    fig, ax = plt.subplots(figsize=(10, 8))
    
    # Prepare data for heatmap
    cat_cols = ['F1_Single', 'F1_Multi', 'F1_Temporal', 'F1_Open']
    df_heatmap = df[['Strategy'] + cat_cols].set_index('Strategy')
    df_heatmap.columns = ['Single-hop', 'Multi-hop', 'Temporal', 'Open-domain']
    
    # Sort by average
    df_heatmap = df_heatmap.loc[df_heatmap.mean(axis=1).sort_values(ascending=False).index]
    
    # Create heatmap
    sns.heatmap(df_heatmap, annot=True, fmt='.1f', cmap='YlOrRd', 
                cbar_kws={'label': 'F1 Score (%)'}, linewidths=0.5,
                vmin=20, vmax=65, ax=ax)
    
    ax.set_title('MemSt Performance by Question Category', fontweight='bold', fontsize=14, pad=20)
    ax.set_xlabel('Question Category', fontweight='bold', fontsize=12)
    ax.set_ylabel('Strategy', fontweight='bold', fontsize=12)
    
    plt.tight_layout()
    plt.savefig(output_dir / 'figure2_category_heatmap.png', bbox_inches='tight')
    plt.savefig(output_dir / 'figure2_category_heatmap.pdf', bbox_inches='tight')
    print("✓ Figure 2: Category Heatmap")
    plt.close()

def figure_3_latency_accuracy_scatter(df, output_dir):
    """Figure 3: Latency vs Accuracy Tradeoff"""
    fig, ax = plt.subplots(figsize=(10, 7))
    
    # Create scatter plot
    for _, row in df.iterrows():
        strategy = row['Strategy']
        color = COLORS.get(strategy.lower().replace('-', '_').replace(' ', '_'), '#333333')
        
        ax.scatter(row['Latency_p50'], row['F1'], s=row['Tokens']/10, 
                  c=color, alpha=0.7, edgecolors='black', linewidth=1.5,
                  label=strategy)
        
        # Add labels
        offset_x = 5 if row['Latency_p50'] < 150 else -5
        offset_y = 1.5 if row['F1'] < 40 else -1.5
        ha = 'left' if offset_x > 0 else 'right'
        ax.annotate(strategy.replace('-', '\n'), 
                   (row['Latency_p50'], row['F1']),
                   xytext=(offset_x, offset_y), textcoords='offset points',
                   fontsize=8, ha=ha, fontweight='bold',
                   bbox=dict(boxstyle='round,pad=0.3', facecolor='white', alpha=0.7))
    
    # Add ideal region
    ax.axhspan(45, 50, alpha=0.1, color='green')
    ax.axvspan(50, 150, alpha=0.1, color='green')
    ax.text(100, 47, 'Target Zone\n(High Acc, Low Lat)', ha='center', fontsize=9, 
            style='italic', color='darkgreen')
    
    ax.set_xlabel('Latency (ms, p50)', fontweight='bold', fontsize=12)
    ax.set_ylabel('F1 Score (%)', fontweight='bold', fontsize=12)
    ax.set_title('Accuracy vs Latency Tradeoff\n(Bubble size = Token count)', 
                fontweight='bold', fontsize=14, pad=20)
    ax.grid(True, alpha=0.3)
    ax.set_xlim(50, 350)
    ax.set_ylim(20, 52)
    
    plt.tight_layout()
    plt.savefig(output_dir / 'figure3_latency_accuracy.png', bbox_inches='tight')
    plt.savefig(output_dir / 'figure3_latency_accuracy.pdf', bbox_inches='tight')
    print("✓ Figure 3: Latency vs Accuracy")
    plt.close()

def figure_4_category_comparison(df, output_dir):
    """Figure 4: Grouped Bar Chart by Category"""
    fig, ax = plt.subplots(figsize=(12, 7))
    
    categories = ['Single-hop', 'Multi-hop', 'Temporal', 'Open-domain']
    cat_ids = ['F1_Single', 'F1_Multi', 'F1_Temporal', 'F1_Open']
    
    # Select top strategies
    top_strategies = df.nlargest(6, 'F1')['Strategy'].tolist()
    df_top = df[df['Strategy'].isin(top_strategies)]
    
    x = np.arange(len(categories))
    width = 0.12
    
    for i, (_, row) in enumerate(df_top.iterrows()):
        values = [row[c] for c in cat_ids]
        offset = (i - len(df_top)/2) * width
        color = COLORS.get(row['Strategy'].lower().replace('-', '_').replace(' ', '_'), '#333333')
        ax.bar(x + offset, values, width, label=row['Strategy'], 
               color=color, edgecolor='black', linewidth=0.5)
    
    ax.set_xlabel('Question Category', fontweight='bold', fontsize=12)
    ax.set_ylabel('F1 Score (%)', fontweight='bold', fontsize=12)
    ax.set_title('Performance Comparison by Question Category', fontweight='bold', fontsize=14, pad=20)
    ax.set_xticks(x)
    ax.set_xticklabels(categories)
    ax.legend(loc='upper right', fontsize=9)
    ax.grid(axis='y', alpha=0.3)
    ax.set_ylim(0, 70)
    
    plt.tight_layout()
    plt.savefig(output_dir / 'figure4_category_comparison.png', bbox_inches='tight')
    plt.savefig(output_dir / 'figure4_category_comparison.pdf', bbox_inches='tight')
    print("✓ Figure 4: Category Comparison")
    plt.close()

def figure_5_token_efficiency(df, output_dir):
    """Figure 5: Token Efficiency (F1 per Token)"""
    fig, ax = plt.subplots(figsize=(10, 6))
    
    df['F1_per_1000_tokens'] = df['F1'] / (df['Tokens'] / 1000)
    df_sorted = df.sort_values('F1_per_1000_tokens', ascending=True)
    
    colors = [COLORS.get(s.lower().replace('-', '_').replace(' ', '_'), '#333333') 
              for s in df_sorted['Strategy']]
    
    bars = ax.barh(df_sorted['Strategy'], df_sorted['F1_per_1000_tokens'], 
                   color=colors, edgecolor='black', linewidth=0.5)
    
    for bar, val in zip(bars, df_sorted['F1_per_1000_tokens']):
        ax.text(val + 0.5, bar.get_y() + bar.get_height()/2, 
                f'{val:.1f}', va='center', fontsize=10, fontweight='bold')
    
    ax.set_xlabel('F1 Score per 1000 Tokens', fontweight='bold', fontsize=12)
    ax.set_title('Token Efficiency (Higher is Better)', fontweight='bold', fontsize=14, pad=20)
    ax.grid(axis='x', alpha=0.3)
    
    plt.tight_layout()
    plt.savefig(output_dir / 'figure5_token_efficiency.png', bbox_inches='tight')
    plt.savefig(output_dir / 'figure5_token_efficiency.pdf', bbox_inches='tight')
    print("✓ Figure 5: Token Efficiency")
    plt.close()

def figure_6_radar_chart(df, output_dir):
    """Figure 6: Radar Chart for Top Strategies"""
    from math import pi
    
    fig, ax = plt.subplots(figsize=(9, 9), subplot_kw=dict(projection='polar'))
    
    categories = ['Single-hop', 'Multi-hop', 'Temporal', 'Open-domain']
    cat_cols = ['F1_Single', 'F1_Multi', 'F1_Temporal', 'F1_Open']
    
    # Number of variables
    N = len(categories)
    angles = [n / float(N) * 2 * pi for n in range(N)]
    angles += angles[:1]
    
    # Plot top 4 strategies
    top_strategies = df.nlargest(4, 'F1')
    
    for _, row in top_strategies.iterrows():
        values = [row[c] for c in cat_cols]
        values += values[:1]
        
        color = COLORS.get(row['Strategy'].lower().replace('-', '_').replace(' ', '_'), '#333333')
        ax.plot(angles, values, 'o-', linewidth=2, label=row['Strategy'], color=color)
        ax.fill(angles, values, alpha=0.15, color=color)
    
    ax.set_xticks(angles[:-1])
    ax.set_xticklabels(categories, fontsize=11)
    ax.set_ylim(0, 65)
    ax.set_title('Strategy Performance Profile\n(Radar Chart)', 
                fontweight='bold', fontsize=14, pad=30, y=1.08)
    ax.legend(loc='upper right', bbox_to_anchor=(1.3, 1.1), fontsize=10)
    ax.grid(True)
    
    plt.tight_layout()
    plt.savefig(output_dir / 'figure6_radar_chart.png', bbox_inches='tight')
    plt.savefig(output_dir / 'figure6_radar_chart.pdf', bbox_inches='tight')
    print("✓ Figure 6: Radar Chart")
    plt.close()

def generate_all_figures():
    """Generate all benchmark figures."""
    output_dir = Path('../results/figures')
    output_dir.mkdir(exist_ok=True)
    
    print("="*60)
    print("Generating MemSt Benchmark Figures")
    print("="*60)
    
    results = load_results()
    df = prepare_data(results)
    
    print(f"\nLoaded {len(df)} strategies")
    print(f"Output directory: {output_dir}")
    print()
    
    figure_1_overall_performance(df, output_dir)
    figure_2_category_heatmap(df, output_dir)
    figure_3_latency_accuracy_scatter(df, output_dir)
    figure_4_category_comparison(df, output_dir)
    figure_5_token_efficiency(df, output_dir)
    figure_6_radar_chart(df, output_dir)
    
    print("\n" + "="*60)
    print("All figures generated successfully!")
    print("="*60)
    print(f"\nFigures saved to: {output_dir}")
    print("\nGenerated files:")
    for f in sorted(output_dir.glob('figure*')):
        print(f"  - {f.name}")

if __name__ == "__main__":
    generate_all_figures()
