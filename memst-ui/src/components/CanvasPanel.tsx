import React from 'react';
import { useApp } from '../context/AppContext';
import { Memory, Trace, File as FileType, KnowledgeGraph } from '../types';
import * as d3 from 'd3';

interface MemoryTierProps {
  tier: 'working' | 'short' | 'long';
  memories: Memory[];
}

function MemoryTier({ tier, memories }: MemoryTierProps) {
  const tierConfig = {
    working: { name: 'Working Memory', icon: 'fa-bolt', color: 'working', count: memories.length },
    short: { name: 'Short-term Memory', icon: 'fa-clock', color: 'short', count: memories.length },
    long: { name: 'Long-term Memory', icon: 'fa-database', color: 'long', count: memories.length },
  };

  const config = tierConfig[tier];

  if (memories.length === 0) {
    return (
      <div className="memory-section">
        <div className="memory-tier">
          <div className={`memory-tier-icon ${config.color}`}>
            <i className={`fas ${config.icon}`}></i>
          </div>
          <span className="memory-tier-name">{config.name}</span>
          <span className="memory-tier-count">0 items</span>
        </div>
        <div className="empty-tier-message">
          No {tier} memories yet
        </div>
      </div>
    );
  }

  return (
    <div className="memory-section">
      <div className="memory-tier">
        <div className={`memory-tier-icon ${config.color}`}>
          <i className={`fas ${config.icon}`}></i>
        </div>
        <span className="memory-tier-name">{config.name}</span>
        <span className="memory-tier-count">{config.count} items</span>
      </div>
      {memories.map((memory) => (
        <div key={memory.id} className="memory-item">
          <div className="memory-item-text">{memory.content}</div>
          {memory.round !== undefined && (
            <div className="memory-round-badge">Round {memory.round}</div>
          )}
          <div className="memory-item-meta">
            {memory.timestamp && <span>{formatTime(memory.timestamp)}</span>}
          </div>
        </div>
      ))}
    </div>
  );
}

function formatTime(isoString: string): string {
  if (!isoString) return '';
  const date = new Date(isoString);
  const now = new Date();
  const diff = now.getTime() - date.getTime();
  const minutes = Math.floor(diff / 60000);
  const hours = Math.floor(diff / 3600000);
  const days = Math.floor(diff / 86400000);

  if (minutes < 1) return 'now';
  if (minutes < 60) return `${minutes}m ago`;
  if (hours < 24) return `${hours}h ago`;
  return `${days}d ago`;
}

interface TraceItemProps {
  trace: Trace;
}

function TraceItem({ trace }: TraceItemProps) {
  const typeIcons: Record<string, string> = {
    memory_retrieval: 'fa-database',
    context_assembly: 'fa-cogs',
    llm_inference: 'fa-microchip',
    tool: 'fa-wrench',
    thinking: 'fa-brain',
    api: 'fa-exchange-alt',
    error: 'fa-exclamation-triangle',
  };

  return (
    <div className="trace-item">
      <div className={`trace-icon ${trace.status === 'error' ? 'error' : trace.type.split('_')[0]}`}>
        <i className={`fas ${typeIcons[trace.type] || 'fa-circle'}`}></i>
      </div>
      <div className="trace-content">
        <div className="trace-title">{trace.title}</div>
        <div className="trace-detail">{trace.detail}</div>
      </div>
      <div className="trace-time">{formatTime(trace.timestamp)}</div>
    </div>
  );
}

interface FileListProps {
  files: FileType[];
}

function FileList({ files }: FileListProps) {
  const typeIcons: Record<string, string> = {
    pdf: 'fa-file-pdf',
    doc: 'fa-file-alt',
    img: 'fa-file-image',
    code: 'fa-file-code',
  };

  return (
    <div className="resource-list">
      {files.map((file) => (
        <div key={file.id} className="resource-item">
          <div className={`resource-icon ${file.type}`}>
            <i className={`fas ${typeIcons[file.type] || 'fa-file'}`}></i>
          </div>
          <div className="resource-name">{file.name}</div>
          <div className="resource-size">{formatSize(file.size)}</div>
        </div>
      ))}
    </div>
  );
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${Math.round(bytes / (1024 * 1024))} MB`;
}

interface KnowledgeGraphViewProps {
  graph: KnowledgeGraph | null;
}

interface D3Node extends d3.SimulationNodeDatum {
  id: string;
  label: string;
  type: string;
  tier: string;
}

interface D3Edge extends d3.SimulationLinkDatum<D3Node> {
  source: string | D3Node;
  target: string | D3Node;
  label?: string;
}

function KnowledgeGraphView({ graph }: KnowledgeGraphViewProps) {
  const [loading, setLoading] = React.useState(false);
  const { state, parseKnowledgeGraph } = useApp();
  const [selectedTiers, setSelectedTiers] = React.useState<string[]>(['working', 'long']);
  const [showDropdown, setShowDropdown] = React.useState(false);
  const [showModal, setShowModal] = React.useState(false);
  const [selectedNode, setSelectedNode] = React.useState<D3Node | null>(null);
  const [zoomLevel, setZoomLevel] = React.useState(100);

  const svgRef = React.useRef<SVGSVGElement>(null);
  const simulationRef = React.useRef<d3.Simulation<D3Node, D3Edge> | null>(null);

  // Use local state that syncs with prop
  const [localGraph, setLocalGraph] = React.useState(graph);
  const [parsed, setParsed] = React.useState(false);

  // Sync with prop when it changes
  React.useEffect(() => {
    setLocalGraph(graph);
    if (graph && graph.nodes.length > 0) {
      setParsed(true);
    }
  }, [graph]);

  // Color scheme by tier (Palantir-inspired)
  const tierColors: Record<string, { fill: string; stroke: string; text: string; icon: string }> = {
    working: { fill: '#e3f2fd', stroke: '#1976d2', text: '#1565c0', icon: 'fa-bolt' },
    short: { fill: '#fff3e0', stroke: '#f57c00', text: '#e65100', icon: 'fa-clock' },
    long: { fill: '#e8f5e9', stroke: '#388e3c', text: '#2e7d32', icon: 'fa-database' },
  };

  const defaultColors = { fill: '#f5f5f5', stroke: '#757575', text: '#616161', icon: 'fa-circle' };

  const toggleTier = (tier: string) => {
    setSelectedTiers(prev => {
      if (prev.includes(tier)) {
        return prev.filter(t => t !== tier);
      } else {
        return [...prev, tier];
      }
    });
  };

  const handleRefresh = async () => {
    if (!state.currentSession) return;
    setLoading(true);
    setSelectedNode(null); // Clear selected node
    try {
      // Use context function which handles API and state update
      await parseKnowledgeGraph(state.currentSession.id);
      setParsed(true);
      // Clear the local graph to trigger re-render with fresh data
      setLocalGraph(null);
      // The useEffect will pick up the new graph from props
      setTimeout(() => {
        setLocalGraph(state.knowledgeGraph);
      }, 100);
    } catch (err) {
      console.error('Failed to parse knowledge graph:', err);
    } finally {
      setLoading(false);
    }
  };

  const handleNodeClick = (node: D3Node) => {
    setSelectedNode(node);
  };

  const handleExpand = () => {
    setShowModal(true);
  };

  // Initialize D3 force simulation
  React.useEffect(() => {
    if (!localGraph || localGraph.nodes.length === 0 || !svgRef.current) return;

    const svg = d3.select(svgRef.current);
    svg.selectAll('*').remove();

    const width = 280;
    const height = 220;

    // Create zoom behavior
    const zoom = d3.zoom<SVGSVGElement, unknown>()
      .scaleExtent([0.3, 3])
      .on('zoom', (event) => {
        containerGroup.attr('transform', event.transform);
        setZoomLevel(Math.round(event.transform.k * 100));
      });

    svg.call(zoom);

    const containerGroup = svg.append('g');

    // Create arrow marker
    svg.append('defs').append('marker')
      .attr('id', 'arrowhead')
      .attr('viewBox', '-0 -5 10 10')
      .attr('refX', 20)
      .attr('refY', 0)
      .attr('orient', 'auto')
      .attr('markerWidth', 6)
      .attr('markerHeight', 6)
      .append('path')
      .attr('d', 'M 0,-5 L 10,0 L 0,5')
      .attr('fill', '#94a3b8');

    // Convert to D3 nodes and edges
    const nodes: D3Node[] = localGraph.nodes.map(n => ({
      id: n.id,
      label: n.label,
      type: n.type,
      tier: n.tier || 'working',
    }));

    const edges: D3Edge[] = localGraph.edges.map(e => ({
      source: e.from,
      target: e.to,
      label: e.label,
    }));

    // Create force simulation
    const simulation = d3.forceSimulation<D3Node>(nodes)
      .force('link', d3.forceLink<D3Node, D3Edge>(edges)
        .id(d => d.id)
        .distance(60)
        .strength(0.5))
      .force('charge', d3.forceManyBody().strength(-150))
      .force('center', d3.forceCenter(width / 2, height / 2))
      .force('collision', d3.forceCollide().radius(25))
      .force('x', d3.forceX(width / 2).strength(0.05))
      .force('y', d3.forceY(height / 2).strength(0.05));

    simulationRef.current = simulation;

    // Create edge elements
    const link = containerGroup.append('g')
      .selectAll('line')
      .data(edges)
      .enter()
      .append('line')
      .attr('class', 'kg-edge')
      .attr('stroke', '#cbd5e1')
      .attr('stroke-width', 1.5)
      .attr('marker-end', 'url(#arrowhead)');

    // Create node groups
    const node = containerGroup.append('g')
      .selectAll('.kg-node-group')
      .data(nodes)
      .enter()
      .append('g')
      .attr('class', 'kg-node-group')
      .style('cursor', 'grab')
      .call(d3.drag<SVGGElement, D3Node>()
        .on('start', (event, d) => {
          if (!event.active) simulation.alphaTarget(0.3).restart();
          d.fx = d.x;
          d.fy = d.y;
          d3.select(event.sourceEvent.target.parentNode).style('cursor', 'grabbing');
        })
        .on('drag', (event, d) => {
          d.fx = event.x;
          d.fy = event.y;
        })
        .on('end', (event, d) => {
          if (!event.active) simulation.alphaTarget(0);
          d.fx = null;
          d.fy = null;
          d3.select(event.sourceEvent.target.parentNode).style('cursor', 'grab');
        }));

    // Node circles
    node.append('circle')
      .attr('class', 'kg-node')
      .attr('r', 18)
      .attr('fill', d => tierColors[d.tier]?.fill || defaultColors.fill)
      .attr('stroke', d => tierColors[d.tier]?.stroke || defaultColors.stroke)
      .attr('stroke-width', 2)
      .style('filter', 'drop-shadow(0 2px 4px rgba(0,0,0,0.1))');

    // Node icons
    node.append('text')
      .attr('class', 'kg-node-icon')
      .attr('text-anchor', 'middle')
      .attr('dy', -2)
      .attr('font-size', '10px')
      .attr('fill', d => tierColors[d.tier]?.stroke || defaultColors.stroke)
      .attr('font-family', 'Font Awesome 6 Free')
      .attr('font-weight', 900)
      .text(d => {
        const icon = tierColors[d.tier]?.icon || defaultColors.icon;
        return icon.replace('fa-', '');
      });

    // Node labels
    node.append('text')
      .attr('class', 'kg-node-label')
      .attr('text-anchor', 'middle')
      .attr('dy', 12)
      .attr('font-size', '9px')
      .attr('fill', d => tierColors[d.tier]?.text || defaultColors.text)
      .attr('font-weight', '500')
      .text(d => d.label.length > 10 ? d.label.slice(0, 8) + '...' : d.label);

    // Click handler
    node.on('click', (event, d) => {
      event.stopPropagation();
      handleNodeClick(d);
    });

    // Update positions on tick
    simulation.on('tick', () => {
      link
        .attr('x1', d => (d.source as D3Node).x!)
        .attr('y1', d => (d.source as D3Node).y!)
        .attr('x2', d => (d.target as D3Node).x!)
        .attr('y2', d => (d.target as D3Node).y!);

      node.attr('transform', d => `translate(${d.x},${d.y})`);
    });

    return () => {
      simulation.stop();
    };
  }, [localGraph]);

  // Render controls
  const renderUpdateControls = () => (
    <div className="kg-controls">
      <div className="tier-dropdown">
        <button
          className="btn btn-icon btn-sm"
          onClick={() => setShowDropdown(!showDropdown)}
          title="Select memory tiers"
        >
          <i className="fas fa-filter"></i>
        </button>
        {showDropdown && (
          <div className="dropdown-menu dropdown-below">
            <div className="dropdown-header">Memory Tiers</div>
            <label className="dropdown-item">
              <input
                type="checkbox"
                checked={selectedTiers.includes('working')}
                onChange={() => toggleTier('working')}
              />
              <span className="tier-dot working"></span>
              Working
            </label>
            <label className="dropdown-item">
              <input
                type="checkbox"
                checked={selectedTiers.includes('short')}
                onChange={() => toggleTier('short')}
              />
              <span className="tier-dot short"></span>
              Short-term
            </label>
            <label className="dropdown-item">
              <input
                type="checkbox"
                checked={selectedTiers.includes('long')}
                onChange={() => toggleTier('long')}
              />
              <span className="tier-dot long"></span>
              Long-term
            </label>
          </div>
        )}
      </div>
      <button
        className="btn btn-icon btn-sm"
        onClick={handleRefresh}
        disabled={loading || selectedTiers.length === 0}
        title="Update knowledge graph"
      >
        <i className={`fas ${loading ? 'fa-spinner fa-spin' : 'fa-sync-alt'}`}></i>
      </button>
      <div className="zoom-controls">
        <span className="zoom-level">{zoomLevel}%</span>
        <button className="btn btn-icon btn-sm" onClick={handleExpand} title="Expand view">
          <i className="fas fa-expand-arrows-alt"></i>
        </button>
      </div>
    </div>
  );

  if (!localGraph || localGraph.nodes.length === 0) {
    return (
      <div className="empty-state">
        <i className="fas fa-project-diagram"></i>
        <h3>{parsed ? 'No entities found' : 'No knowledge graph'}</h3>
        <p>{parsed ? 'Add more conversations to extract entities' : 'Knowledge will be extracted from your conversations'}</p>
        {state.currentSession && (
          <div className="kg-controls-panel">
            {renderUpdateControls()}
          </div>
        )}
      </div>
    );
  }

  return (
    <div className="knowledge-graph-container">
      {state.currentSession && renderUpdateControls()}
      <div className="graph-viewport">
        <svg
          ref={svgRef}
          width="280"
          height="220"
          className="kg-svg"
        />
      </div>
      <div className="graph-legend">
        <span className="legend-item">
          <span className="legend-dot working"></span>
          Working
        </span>
        <span className="legend-item">
          <span className="legend-dot short"></span>
          Short
        </span>
        <span className="legend-item">
          <span className="legend-dot long"></span>
          Long
        </span>
        <span className="hint">Drag nodes to reposition</span>
      </div>

      {/* Node Detail Panel */}
      {selectedNode && (
        <div className="node-detail-panel">
          <div className="node-detail-header">
            <span className={`tier-badge ${selectedNode.tier}`}>
              <i className={`fas ${tierColors[selectedNode.tier]?.icon || defaultColors.icon}`}></i>
              {selectedNode.tier}
            </span>
            <button className="btn-close" onClick={() => setSelectedNode(null)}>
              <i className="fas fa-times"></i>
            </button>
          </div>
          <div className="node-detail-content">
            <h4>{selectedNode.label}</h4>
            <p className="node-type">Type: {selectedNode.type}</p>
            <p className="node-id">ID: {selectedNode.id.slice(0, 8)}...</p>
          </div>
          <div className="node-connections">
            <h5>Connections</h5>
            {localGraph.edges
              .filter(e => e.from === selectedNode.id || e.to === selectedNode.id)
              .map((e, i) => {
                const connectedId = e.from === selectedNode.id ? e.to : e.from;
                const connectedNode = localGraph.nodes.find(n => n.id === connectedId);
                return connectedNode ? (
                  <div key={i} className="connection-item">
                    <span className={`connection-dot ${connectedNode.tier}`}></span>
                    {connectedNode.label}
                  </div>
                ) : null;
              })}
          </div>
        </div>
      )}

      {/* Expand Modal */}
      {showModal && (
        <div className="kg-modal-overlay" onClick={() => setShowModal(false)}>
          <div className="kg-modal" onClick={e => e.stopPropagation()}>
            <div className="kg-modal-header">
              <h3>Knowledge Graph Explorer</h3>
              <div className="kg-modal-controls">
                <span className="zoom-level">{zoomLevel}%</span>
                <button className="btn btn-icon" onClick={() => {
                  if (svgRef.current) {
                    d3.select(svgRef.current).call(
                      d3.zoom<SVGSVGElement, unknown>().scaleExtent([0.3, 3]).on('zoom', (event) => {
                        d3.select(svgRef.current!).select('g').attr('transform', event.transform);
                        setZoomLevel(Math.round(event.transform.k * 100));
                      }) as any
                    );
                  }
                }}>
                  <i className="fas fa-compress-arrows-alt"></i>
                </button>
                <button className="btn btn-icon" onClick={() => setShowModal(false)}>
                  <i className="fas fa-times"></i>
                </button>
              </div>
            </div>
            <div className="kg-modal-content">
              <svg
                ref={el => {
                  if (el && localGraph) {
                    const svg = d3.select(el);
                    svg.selectAll('*').remove();

                    const width = 800;
                    const height = 600;

                    const zoom = d3.zoom<SVGSVGElement, unknown>()
                      .scaleExtent([0.3, 3])
                      .on('zoom', (event) => {
                        containerGroup.attr('transform', event.transform);
                        setZoomLevel(Math.round(event.transform.k * 100));
                      });

                    svg.call(zoom);

                    const containerGroup = svg.append('g');

                    svg.append('defs').append('marker')
                      .attr('id', 'arrowhead-modal')
                      .attr('viewBox', '-0 -5 10 10')
                      .attr('refX', 22)
                      .attr('refY', 0)
                      .attr('orient', 'auto')
                      .attr('markerWidth', 6)
                      .attr('markerHeight', 6)
                      .append('path')
                      .attr('d', 'M 0,-5 L 10,0 L 0,5')
                      .attr('fill', '#94a3b8');

                    const nodes: D3Node[] = localGraph.nodes.map(n => ({
                      id: n.id,
                      label: n.label,
                      type: n.type,
                      tier: n.tier || 'working',
                    }));

                    const edges: D3Edge[] = localGraph.edges.map(e => ({
                      source: e.from,
                      target: e.to,
                      label: e.label,
                    }));

                    const simulation = d3.forceSimulation<D3Node>(nodes)
                      .force('link', d3.forceLink<D3Node, D3Edge>(edges)
                        .id(d => d.id)
                        .distance(100)
                        .strength(0.5))
                      .force('charge', d3.forceManyBody().strength(-200))
                      .force('center', d3.forceCenter(width / 2, height / 2))
                      .force('collision', d3.forceCollide().radius(35));

                    const link = containerGroup.append('g')
                      .selectAll('line')
                      .data(edges)
                      .enter()
                      .append('line')
                      .attr('class', 'kg-edge')
                      .attr('stroke', '#cbd5e1')
                      .attr('stroke-width', 2)
                      .attr('marker-end', 'url(#arrowhead-modal)');

                    const node = containerGroup.append('g')
                      .selectAll('.kg-node-group')
                      .data(nodes)
                      .enter()
                      .append('g')
                      .attr('class', 'kg-node-group')
                      .style('cursor', 'grab')
                      .call(d3.drag<SVGGElement, D3Node>()
                        .on('start', (event, d) => {
                          if (!event.active) simulation.alphaTarget(0.3).restart();
                          d.fx = d.x;
                          d.fy = d.y;
                        })
                        .on('drag', (event, d) => {
                          d.fx = event.x;
                          d.fy = event.y;
                        })
                        .on('end', (event, d) => {
                          if (!event.active) simulation.alphaTarget(0);
                          d.fx = null;
                          d.fy = null;
                        }));

                    node.append('circle')
                      .attr('class', 'kg-node')
                      .attr('r', 24)
                      .attr('fill', d => tierColors[d.tier]?.fill || defaultColors.fill)
                      .attr('stroke', d => tierColors[d.tier]?.stroke || defaultColors.stroke)
                      .attr('stroke-width', 2.5)
                      .style('filter', 'drop-shadow(0 3px 6px rgba(0,0,0,0.15))');

                    node.append('text')
                      .attr('class', 'kg-node-icon')
                      .attr('text-anchor', 'middle')
                      .attr('dy', -3)
                      .attr('font-size', '14px')
                      .attr('fill', d => tierColors[d.tier]?.stroke || defaultColors.stroke)
                      .attr('font-family', 'Font Awesome 6 Free')
                      .attr('font-weight', 900)
                      .text(d => {
                        const icon = tierColors[d.tier]?.icon || defaultColors.icon;
                        return icon.replace('fa-', '');
                      });

                    node.append('text')
                      .attr('class', 'kg-node-label')
                      .attr('text-anchor', 'middle')
                      .attr('dy', 16)
                      .attr('font-size', '11px')
                      .attr('fill', d => tierColors[d.tier]?.text || defaultColors.text)
                      .attr('font-weight', '500')
                      .text(d => d.label);

                    simulation.on('tick', () => {
                      link
                        .attr('x1', d => (d.source as D3Node).x!)
                        .attr('y1', d => (d.source as D3Node).y!)
                        .attr('x2', d => (d.target as D3Node).x!)
                        .attr('y2', d => (d.target as D3Node).y!);

                      node.attr('transform', d => `translate(${d.x},${d.y})`);
                    });
                  }
                }}
                width="800"
                height="600"
                className="kg-svg-modal"
              />
            </div>
            <div className="kg-modal-legend">
              <span className="legend-item">
                <span className="legend-dot working"></span>
                Working
              </span>
              <span className="legend-item">
                <span className="legend-dot short"></span>
                Short
              </span>
              <span className="legend-item">
                <span className="legend-dot long"></span>
                Long
              </span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export function CanvasPanel() {
  const { state, dispatch } = useApp();

  const tabs = [
    { key: 'memory', label: 'Memory' },
    { key: 'trace', label: 'Trace' },
    { key: 'files', label: 'Files' },
    { key: 'graph', label: 'Graph' },
  ];

  return (
    <div className="canvas-area">
      <div className="canvas-header">
        <h3>Context Panel</h3>
        <button className="header-btn" title="Expand">
          <i className="fas fa-expand"></i>
        </button>
        <button className="header-btn" title="Close">
          <i className="fas fa-times"></i>
        </button>
      </div>
      <div className="canvas-tabs">
        {tabs.map((tab) => (
          <div
            key={tab.key}
            className={`canvas-tab ${state.activeCanvasTab === tab.key ? 'active' : ''}`}
            onClick={() => dispatch({ type: 'SET_ACTIVE_CANVAS_TAB', payload: tab.key })}
          >
            {tab.label}
          </div>
        ))}
      </div>
      <div className="canvas-content">
        {state.activeCanvasTab === 'memory' && (
          <>
            <MemoryTier tier="working" memories={state.memories.working} />
            <MemoryTier tier="short" memories={state.memories.short} />
            <MemoryTier tier="long" memories={state.memories.long} />
          </>
        )}
        {state.activeCanvasTab === 'trace' && (
          <>
            {state.traces.length === 0 ? (
              <div className="empty-state">
                <i className="fas fa-history"></i>
                <h3>No traces yet</h3>
                <p>Agent operation traces will appear here</p>
              </div>
            ) : (
              state.traces.map((trace) => <TraceItem key={trace.id} trace={trace} />)
            )}
          </>
        )}
        {state.activeCanvasTab === 'files' && <FileList files={state.files} />}
        {state.activeCanvasTab === 'graph' && <KnowledgeGraphView graph={state.knowledgeGraph} />}
      </div>
    </div>
  );
}
