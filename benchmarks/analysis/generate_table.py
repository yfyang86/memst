"""
Generate manuscript-ready results table from benchmark outputs.
"""
import json
import argparse
from pathlib import Path
import sys


def load_summary(path: str) -> dict:
    """Load summary JSON file."""
    with open(path, 'r') as f:
        return json.load(f)


def format_metric(value: float, decimals: int = 2) -> str:
    """Format metric value."""
    return f"{value:.{decimals}f}"


def generate_latex_table(results: dict) -> str:
    """Generate LaTeX table for manuscript."""
    
    strategies = ['full-context', 'rag', 'memst', 'memst-kg']
    categories = [1, 2, 3, 4]  # Skip adversarial (5)
    cat_names = {1: 'Single-Hop', 2: 'Multi-Hop', 3: 'Temporal', 4: 'Open-Domain'}
    
    lines = []
    lines.append("% Auto-generated results table")
    lines.append("\\begin{table*}[t]")
    lines.append("\\centering")
    lines.append("\\caption{Performance comparison on LOCOMO benchmark. J denotes LLM-as-a-Judge score.}")
    lines.append("\\label{tab:locomo_results}")
    lines.append("\\resizebox{\\textwidth}{!}{%")
    
    # Table header
    header = "\\begin{tabular}{l|ccc|ccc|ccc|ccc}"
    lines.append(header)
    lines.append("\\toprule")
    
    # Category headers
    cat_header = "\\multirow{2}{*}{\\textbf{Method}}"
    for cat_id in categories:
        cat_header += f" & \\multicolumn{{3}}{{c|}}{{\\textbf{{{cat_names[cat_id]}}}}}"
    lines.append(cat_header + " \\\\")
    
    # Metric sub-headers
    sub_header = ""
    for _ in categories:
        sub_header += " & F1 $\\uparrow$ & BLEU $\\uparrow$ & J $\\uparrow$"
    lines.append(sub_header + " \\\\")
    lines.append("\\midrule")
    
    # Strategy rows
    for strategy_key in strategies:
        if strategy_key not in results:
            continue
        
        data = results[strategy_key]
        
        # Strategy name mapping
        name_map = {
            'full-context': 'Full-Context',
            'rag': 'RAG (512-chunk)',
            'memst': 'MemSt',
            'memst-kg': 'MemSt+KG'
        }
        row = name_map.get(strategy_key, strategy_key)
        
        # Add metrics for each category
        for cat_id in categories:
            if str(cat_id) in data.get('by_category', {}):
                cat_data = data['by_category'][str(cat_id)]
                metrics = cat_data['metrics']
                f1 = format_metric(metrics.get('f1', 0) * 100, 2)
                bleu = format_metric(metrics.get('bleu', 0) * 100, 2)
                judge = cat_data.get('judge_mean', 0)
                judge_str = format_metric(judge, 1) if judge else "--"
                row += f" & {f1} & {bleu} & {judge_str}"
            else:
                row += " & -- & -- & --"
        
        lines.append(row + " \\\\")
    
    lines.append("\\bottomrule")
    lines.append("\\end{tabular}%")
    lines.append("}")
    lines.append("\\end{table*}")
    
    return "\n".join(lines)


def generate_markdown_table(results: dict) -> str:
    """Generate Markdown table for README."""
    
    strategies = ['full-context', 'rag', 'memst', 'memst-kg']
    
    lines = []
    lines.append("# MemSt LOCOMO Benchmark Results")
    lines.append("")
    lines.append("## Overall Performance")
    lines.append("")
    lines.append("| Strategy | F1 | BLEU-1 | Avg Latency | Avg Tokens |")
    lines.append("|----------|-----|--------|-------------|------------|")
    
    for strategy_key in strategies:
        if strategy_key not in results:
            continue
        
        data = results[strategy_key]
        name = strategy_key.upper() if strategy_key == 'rag' else strategy_key.title()
        
        metrics = data.get('overall', {})
        f1 = format_metric(metrics.get('f1', 0) * 100, 1)
        bleu = format_metric(metrics.get('bleu', 0) * 100, 1)
        
        op = data.get('operational', {})
        latency = format_metric(op.get('avg_latency', 0), 3)
        tokens = format_metric(op.get('avg_tokens', 0), 0)
        
        lines.append(f"| {name} | {f1} | {bleu} | {latency}s | {tokens} |")
    
    lines.append("")
    lines.append("## By Category (F1 Scores)")
    lines.append("")
    lines.append("| Strategy | Single-Hop | Multi-Hop | Temporal | Open-Domain |")
    lines.append("|----------|------------|-----------|----------|-------------|")
    
    for strategy_key in strategies:
        if strategy_key not in results:
            continue
        
        data = results[strategy_key]
        name = strategy_key.upper() if strategy_key == 'rag' else strategy_key.title()
        
        row = f"| {name}"
        for cat_id in [1, 2, 3, 4]:
            cat_data = data.get('by_category', {}).get(str(cat_id), {})
            metrics = cat_data.get('metrics', {})
            f1 = format_metric(metrics.get('f1', 0) * 100, 1)
            row += f" | {f1}"
        
        lines.append(row + " |")
    
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description='Generate results tables')
    parser.add_argument('summaries', nargs='+', help='Summary JSON files')
    parser.add_argument('--output', '-o', default='results/table.tex',
                       help='Output file')
    parser.add_argument('--format', choices=['latex', 'markdown'], default='markdown',
                       help='Output format')
    
    args = parser.parse_args()
    
    # Load all results
    results = {}
    for path in args.summaries:
        p = Path(path)
        if p.exists():
            # Extract strategy name from filename
            strategy = p.stem.replace('.jsonl', '')
            results[strategy] = load_summary(path)
            print(f"Loaded: {strategy}")
        else:
            print(f"Warning: File not found: {path}")
    
    if not results:
        print("No results to process")
        sys.exit(1)
    
    # Generate table
    if args.format == 'latex':
        table = generate_latex_table(results)
    else:
        table = generate_markdown_table(results)
    
    # Save output
    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    
    with open(output_path, 'w') as f:
        f.write(table)
    
    print(f"\nTable saved to: {output_path}")
    print("\n" + "="*60)
    print(table)


if __name__ == "__main__":
    main()
