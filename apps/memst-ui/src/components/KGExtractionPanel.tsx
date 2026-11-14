import React from 'react';
import { useApp } from '../context/AppContext';
import { Ontology, ExtractedEntity, ExtractionJob } from '../types';

const DEFAULT_SCHEMA = `[
  {
    "top_category": "领域情报类",
    "first_category": "科技情报",
    "second_category": "人工智能",
    "chinese_name": "科技情报-人工智能",
    "english_name": "Tech Intelligence-AI",
    "overview": "AI技术、产品、公司、人物相关情报"
  },
  {
    "top_category": "领域情报类",
    "first_category": "科技情报",
    "second_category": "半导体芯片",
    "chinese_name": "科技情报-半导体芯片",
    "english_name": "Tech Intelligence-Semiconductor",
    "overview": "芯片设计、制造、封测全产业链"
  }
]`;

export function KGExtractionPanel() {
  const { state, loadOntologyFromSchema, extractEntities, extractFromSession, searchEntities, loadOntologies } = useApp();
  
  const [schemaInput, setSchemaInput] = React.useState(DEFAULT_SCHEMA);
  const [selectedOntology, setSelectedOntology] = React.useState<string>('');
  const [extractText, setExtractText] = React.useState('');
  const [searchQuery, setSearchQuery] = React.useState('');
  const [loading, setLoading] = React.useState(false);
  const [activeTab, setActiveTab] = React.useState<'ontologies' | 'extract' | 'entities'>('ontologies');
  const [lastJob, setLastJob] = React.useState<ExtractionJob | null>(null);

  const handleLoadOntologies = async () => {
    setLoading(true);
    try {
      const ids = await loadOntologyFromSchema(schemaInput);
      if (ids.length > 0) {
        setSelectedOntology(ids[0]);
      }
    } finally {
      setLoading(false);
    }
  };

  const handleExtract = async () => {
    if (!selectedOntology || !extractText) return;
    setLoading(true);
    try {
      const job = await extractEntities(`doc-${Date.now()}`, extractText, selectedOntology);
      if (job) {
        setLastJob(job);
      }
    } finally {
      setLoading(false);
    }
  };

  const handleExtractFromSession = async () => {
    if (!state.currentSession || !selectedOntology) return;
    setLoading(true);
    try {
      const result = await extractFromSession(state.currentSession.id, selectedOntology);
      if (result) {
        alert(`Extracted from ${result.messages_processed} messages using ${result.total_tokens_used} tokens`);
      }
    } finally {
      setLoading(false);
    }
  };

  const handleSearch = async () => {
    setLoading(true);
    try {
      await searchEntities(searchQuery, 20);
    } finally {
      setLoading(false);
    }
  };

  const formatConfidence = (confidence: number): string => {
    return `${(confidence * 100).toFixed(1)}%`;
  };

  const getStatusColor = (status: string): string => {
    switch (status) {
      case 'completed': return 'success';
      case 'running': return 'warning';
      case 'failed': return 'error';
      default: return 'default';
    }
  };

  return (
    <div className="kg-extraction-panel">
      {/* Header */}
      <div className="kg-header">
        <div className="kg-status-badge">
          <span className={`status-dot ${state.kgStatus?.available ? 'active' : 'inactive'}`}></span>
          KG Extraction v2 {state.kgStatus?.available ? 'Available' : 'Unavailable'}
        </div>
        {state.kgStatus && (
          <div className="kg-stats">
            {state.kgStatus.ontologies_loaded} ontologies loaded
          </div>
        )}
      </div>

      {/* Tabs */}
      <div className="kg-tabs">
        <button
          className={`kg-tab ${activeTab === 'ontologies' ? 'active' : ''}`}
          onClick={() => setActiveTab('ontologies')}
        >
          <i className="fas fa-sitemap"></i>
          Ontologies
        </button>
        <button
          className={`kg-tab ${activeTab === 'extract' ? 'active' : ''}`}
          onClick={() => setActiveTab('extract')}
        >
          <i className="fas fa-bolt"></i>
          Extract
        </button>
        <button
          className={`kg-tab ${activeTab === 'entities' ? 'active' : ''}`}
          onClick={() => setActiveTab('entities')}
        >
          <i className="fas fa-cubes"></i>
          Entities ({state.extractedEntities.length})
        </button>
      </div>

      {/* Ontologies Tab */}
      {activeTab === 'ontologies' && (
        <div className="kg-section">
          <div className="kg-section-title">
            <i className="fas fa-code"></i>
            Load Ontologies from Schema
          </div>
          <textarea
            className="kg-schema-input"
            value={schemaInput}
            onChange={(e) => setSchemaInput(e.target.value)}
            placeholder="Paste ontology schema JSON here..."
            rows={8}
          />
          <button
            className="btn btn-primary btn-sm"
            onClick={handleLoadOntologies}
            disabled={loading || !schemaInput.trim()}
          >
            {loading ? <i className="fas fa-spinner fa-spin"></i> : <i className="fas fa-upload"></i>}
            Load Ontologies
          </button>

          {state.ontologies.length > 0 && (
            <div className="kg-ontologies-list">
              <div className="kg-section-title">
                <i className="fas fa-list"></i>
                Loaded Ontologies
              </div>
              {state.ontologies.map((ont: Ontology) => (
                <div
                  key={ont.id}
                  className={`kg-ontology-card ${selectedOntology === ont.id ? 'selected' : ''}`}
                  onClick={() => setSelectedOntology(ont.id)}
                >
                  <div className="kg-ontology-header">
                    <span className="kg-ontology-name">{ont.english_name}</span>
                    <span className="kg-ontology-id">{ont.id}</span>
                  </div>
                  <div className="kg-ontology-categories">
                    <span className="category-tag">{ont.top_category}</span>
                    <span className="category-tag">{ont.first_category}</span>
                    <span className="category-tag">{ont.second_category}</span>
                  </div>
                  {ont.overview && (
                    <div className="kg-ontology-overview">{ont.overview}</div>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Extract Tab */}
      {activeTab === 'extract' && (
        <div className="kg-section">
          <div className="kg-section-title">
            <i className="fas fa-align-left"></i>
            Text to Extract
          </div>
          
          {state.ontologies.length === 0 && (
            <div className="kg-warning">
              <i className="fas fa-exclamation-triangle"></i>
              No ontologies loaded. Please load ontologies first.
            </div>
          )}

          {state.ontologies.length > 0 && (
            <>
              <select
                className="kg-select"
                value={selectedOntology}
                onChange={(e) => setSelectedOntology(e.target.value)}
              >
                <option value="">Select an ontology...</option>
                {state.ontologies.map((ont: Ontology) => (
                  <option key={ont.id} value={ont.id}>
                    {ont.english_name} ({ont.id})
                  </option>
                ))}
              </select>

              <textarea
                className="kg-text-input"
                value={extractText}
                onChange={(e) => setExtractText(e.target.value)}
                placeholder="Enter text to extract entities from..."
                rows={6}
              />

              <div className="kg-actions">
                <button
                  className="btn btn-primary btn-sm"
                  onClick={handleExtract}
                  disabled={loading || !selectedOntology || !extractText.trim()}
                >
                  {loading ? <i className="fas fa-spinner fa-spin"></i> : <i className="fas fa-bolt"></i>}
                  Extract Entities
                </button>
                
                {state.currentSession && (
                  <button
                    className="btn btn-secondary btn-sm"
                    onClick={handleExtractFromSession}
                    disabled={loading || !selectedOntology}
                  >
                    <i className="fas fa-comments"></i>
                    Extract from Session
                  </button>
                )}
              </div>

              {lastJob && (
                <div className={`kg-job-result ${getStatusColor(lastJob.status)}`}>
                  <div className="kg-job-header">
                    <i className="fas fa-check-circle"></i>
                    Extraction Complete
                  </div>
                  <div className="kg-job-stats">
                    <div className="stat">
                      <span className="stat-value">{lastJob.entity_count}</span>
                      <span className="stat-label">Entities</span>
                    </div>
                    <div className="stat">
                      <span className="stat-value">{lastJob.relationship_count}</span>
                      <span className="stat-label">Relationships</span>
                    </div>
                    <div className="stat">
                      <span className="stat-value">{lastJob.tokens_used}</span>
                      <span className="stat-label">Tokens</span>
                    </div>
                  </div>
                </div>
              )}
            </>
          )}
        </div>
      )}

      {/* Entities Tab */}
      {activeTab === 'entities' && (
        <div className="kg-section">
          <div className="kg-search-bar">
            <input
              type="text"
              className="kg-search-input"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Search extracted entities..."
              onKeyPress={(e) => e.key === 'Enter' && handleSearch()}
            />
            <button
              className="btn btn-icon btn-sm"
              onClick={handleSearch}
              disabled={loading}
            >
              {loading ? <i className="fas fa-spinner fa-spin"></i> : <i className="fas fa-search"></i>}
            </button>
          </div>

          {state.extractedEntities.length === 0 ? (
            <div className="kg-empty">
              <i className="fas fa-cubes"></i>
              <p>No extracted entities yet</p>
              <span>Extract entities from text or sessions to see them here</span>
            </div>
          ) : (
            <div className="kg-entities-grid">
              {state.extractedEntities.map((entity: ExtractedEntity, index: number) => (
                <div key={entity.id || index} className="kg-entity-card">
                  <div className="kg-entity-header">
                    <span className="kg-entity-name">{entity.name}</span>
                    <span className="kg-entity-confidence">
                      {formatConfidence(entity.confidence)}
                    </span>
                  </div>
                  <div className="kg-entity-type">
                    <i className="fas fa-tag"></i>
                    {entity.entity_type}
                  </div>
                  {entity.ontology_id && (
                    <div className="kg-entity-ontology">
                      <i className="fas fa-sitemap"></i>
                      {entity.ontology_id}
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
