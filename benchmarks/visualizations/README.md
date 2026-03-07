# MemSt Benchmark Visualizations

Publication-quality figures for the MemSt paper, generated using Python (matplotlib + seaborn).

## Generated Figures

### Figure 1: Overall Performance Comparison
**File**: `figure1_overall_performance.png/pdf`

Horizontal bar chart showing F1 scores for all strategies, sorted by performance.
- Highlights QuestionAdaptive as best overall (49.2% F1)
- Shows 118% improvement over RAG baseline
- Includes RAG baseline reference line

### Figure 2: Category Performance Heatmap
**File**: `figure2_category_heatmap.png/pdf`

Color-coded heatmap showing F1 scores across question categories.
- Rows: Strategies (sorted by average performance)
- Columns: Question categories (Single-hop, Multi-hop, Temporal, Open-domain)
- Color intensity: Yellow (high) to Red (low)
- Annotated with exact F1 scores

### Figure 3: Latency vs Accuracy Tradeoff
**File**: `figure3_latency_accuracy.png/pdf`

Scatter plot showing the accuracy-latency tradeoff.
- X-axis: Latency (ms, p50)
- Y-axis: F1 Score (%)
- Bubble size: Token count
- Green shaded region: Target zone (high accuracy, low latency)
- Each strategy labeled

### Figure 4: Category Comparison (Grouped Bar)
**File**: `figure4_category_comparison.png/pdf`

Grouped bar chart comparing top 6 strategies across categories.
- Easy visual comparison per category
- Shows which strategies excel in which domains
- Color-coded by strategy

### Figure 5: Token Efficiency
**File**: `figure5_token_efficiency.png/pdf`

Horizontal bar chart showing F1 score per 1000 tokens.
- Measures efficiency (higher is better)
- Shows which strategies get most accuracy per token
- Useful for resource-constrained deployments

### Figure 6: Strategy Performance Profile (Radar Chart)
**File**: `figure6_radar_chart.png/pdf`

Radar/spider chart showing performance across all categories.
- Compares top 4 strategies
- Shape shows strengths/weaknesses
- Larger area = better overall performance
- Useful for strategy selection

## Usage

### Generate All Figures
```bash
cd benchmarks/visualizations
python3 generate_figures.py
```

### Output Location
All figures saved to: `../results/figures/`

Format: PNG (web) and PDF (print/publication)

### Requirements
```bash
pip install matplotlib seaborn pandas numpy
```

## Customization

### Colors
Edit the `COLORS` dictionary in `generate_figures.py`:
```python
COLORS = {
    'question_adaptive': '#2E86AB',  # Blue
    'hierarchical': '#A23B72',       # Purple
    # ...
}
```

### Styles
Modify seaborn settings at the top of the script:
```python
sns.set_style("whitegrid")  # or "darkgrid", "ticks", etc.
sns.set_context("paper", font_scale=1.3)  # or "talk", "poster"
```

### Resolution
Change DPI settings:
```python
plt.rcParams['figure.dpi'] = 300
plt.rcParams['savefig.dpi'] = 300  # Higher for publication
```

## Data Source

Figures are generated from: `../results/optimized_comparison.json`

This file contains benchmark results for all strategies across all metrics.

## Publication Notes

- All figures use 300 DPI for publication quality
- PDFs are vector-based (scalable without loss)
- Font: Arial/DejaVu Sans (standard scientific font)
- Colors are colorblind-friendly
- Figures suitable for 2-column academic papers
