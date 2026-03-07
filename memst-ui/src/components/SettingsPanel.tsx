import { useState } from 'react';
import { useSettings } from '../context/SettingsContext';

interface SettingsPanelProps {
  isOpen: boolean;
  onClose: () => void;
}

type Tab = 'llm' | 'embedding' | 'server' | 'kg-extraction';

export function SettingsPanel({ isOpen, onClose }: SettingsPanelProps) {
  const {
    settings,
    updateLLM,
    updateEmbedding,
    updateKGExtraction,
    updateServer,
    resetSettings,
    saveSettings,
    isLoading,
    error,
  } = useSettings();
  const [activeTab, setActiveTab] = useState<Tab>('llm');
  const [isSaving, setIsSaving] = useState(false);

  if (!isOpen) return null;

  const tabs: { id: Tab; label: string; icon: string }[] = [
    { id: 'llm', label: 'LLM', icon: 'fa-brain' },
    { id: 'embedding', label: 'Embedding', icon: 'fa-vector-square' },
    { id: 'server', label: 'Server', icon: 'fa-server' },
    { id: 'kg-extraction', label: 'KG Extraction', icon: 'fa-project-diagram' },
  ];

  const handleSave = async () => {
    setIsSaving(true);
    await saveSettings();
    setIsSaving(false);
    onClose();
  };

  const handleReset = async () => {
    if (window.confirm('Reset all settings to default?')) {
      await resetSettings();
    }
  };

  if (isLoading) {
    return (
      <div className="modal-overlay active">
        <div className="modal">
          <div className="modal-body">
            <div className="empty-state">
              <i className="fas fa-spinner fa-spin"></i>
              <h3>Loading settings...</h3>
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="modal-overlay active" onClick={onClose}>
      <div className="modal" style={{ maxWidth: '600px' }} onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>Settings</h2>
          <button className="modal-close" onClick={onClose}>
            <i className="fas fa-times"></i>
          </button>
        </div>
        <div style={{ display: 'flex', borderBottom: '1px solid var(--border)' }}>
          {tabs.map((tab) => (
            <div
              key={tab.id}
              className={`canvas-tab ${activeTab === tab.id ? 'active' : ''}`}
              style={{ flex: 1, textAlign: 'center' }}
              onClick={() => setActiveTab(tab.id)}
            >
              <i className={`fas ${tab.icon}`} style={{ marginRight: '6px' }}></i>
              {tab.label}
            </div>
          ))}
        </div>
        <div className="modal-body" style={{ maxHeight: '60vh', overflowY: 'auto' }}>
          {error && (
            <div style={{ padding: '8px 12px', background: '#fee2e2', color: '#dc2626', borderRadius: '6px', marginBottom: '12px', fontSize: '13px' }}>
              {error}
            </div>
          )}
          {activeTab === 'llm' && (
            <div>
              <div className="form-group">
                <label className="form-label">Provider Type</label>
                <select
                  className="form-select"
                  value={settings.llm.type}
                  onChange={(e) => updateLLM({ type: e.target.value })}
                >
                  <option value="openai">OpenAI Compatible</option>
                  <option value="anthropic">Anthropic</option>
                  <option value="ollama">Ollama</option>
                  <option value="local">Local Model</option>
                </select>
              </div>
              <div className="form-group">
                <label className="form-label">API URL</label>
                <input
                  type="text"
                  className="form-input"
                  value={settings.llm.api_url}
                  onChange={(e) => updateLLM({ api_url: e.target.value })}
                  placeholder="http://localhost:8080/v1"
                />
              </div>
              <div className="form-group">
                <label className="form-label">Model Name</label>
                <input
                  type="text"
                  className="form-input"
                  value={settings.llm.model}
                  onChange={(e) => updateLLM({ model: e.target.value })}
                  placeholder="gpt-4"
                />
              </div>
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px' }}>
                <div className="form-group">
                  <label className="form-label">Timeout (seconds)</label>
                  <input
                    type="number"
                    className="form-input"
                    value={settings.llm.timeout}
                    onChange={(e) => updateLLM({ timeout: parseInt(e.target.value) || 60 })}
                  />
                </div>
                <div className="form-group">
                  <label className="form-label">Max Tokens</label>
                  <input
                    type="number"
                    className="form-input"
                    value={settings.llm.max_tokens}
                    onChange={(e) => updateLLM({ max_tokens: parseInt(e.target.value) || 8192 })}
                  />
                </div>
              </div>
              <div className="form-group">
                <label className="form-label">Temperature</label>
                <input
                  type="range"
                  min="0"
                  max="2"
                  step="0.1"
                  value={settings.llm.temperature}
                  onChange={(e) => updateLLM({ temperature: parseFloat(e.target.value) })}
                  style={{ width: '100%' }}
                />
                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '12px', color: 'var(--text-muted)' }}>
                  <span>Precise (0)</span>
                  <span>{settings.llm.temperature}</span>
                  <span>Creative (2)</span>
                </div>
              </div>
              <div className="form-group">
                <label className="form-label">API Key (optional)</label>
                <input
                  type="password"
                  className="form-input"
                  value={settings.llm.api_key}
                  onChange={(e) => updateLLM({ api_key: e.target.value })}
                  placeholder="sk-..."
                />
              </div>
            </div>
          )}

          {activeTab === 'embedding' && (
            <div>
              <div className="form-group">
                <label className="form-label">Provider Type</label>
                <select
                  className="form-select"
                  value={settings.embedding.type}
                  onChange={(e) => updateEmbedding({ type: e.target.value })}
                >
                  <option value="openai">OpenAI Compatible</option>
                  <option value="sentence-transformers">Sentence Transformers</option>
                  <option value="huggingface">HuggingFace</option>
                  <option value="local">Local Model</option>
                </select>
              </div>
              <div className="form-group">
                <label className="form-label">API URL</label>
                <input
                  type="text"
                  className="form-input"
                  value={settings.embedding.api_url}
                  onChange={(e) => updateEmbedding({ api_url: e.target.value })}
                  placeholder="http://localhost:8081/v1/embeddings"
                />
              </div>
              <div className="form-group">
                <label className="form-label">Model Name</label>
                <input
                  type="text"
                  className="form-input"
                  value={settings.embedding.model}
                  onChange={(e) => updateEmbedding({ model: e.target.value })}
                  placeholder="text-embedding-bge_m3"
                />
              </div>
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px' }}>
                <div className="form-group">
                  <label className="form-label">Timeout (seconds)</label>
                  <input
                    type="number"
                    className="form-input"
                    value={settings.embedding.timeout}
                    onChange={(e) => updateEmbedding({ timeout: parseInt(e.target.value) || 30 })}
                  />
                </div>
                <div className="form-group">
                  <label className="form-label">Vector Dimension</label>
                  <input
                    type="number"
                    className="form-input"
                    value={settings.embedding.expected_dimension}
                    onChange={(e) => updateEmbedding({ expected_dimension: parseInt(e.target.value) || 1024 })}
                  />
                </div>
              </div>
            </div>
          )}

          {activeTab === 'server' && (
            <div>
              <div className="form-group">
                <label className="form-label">Server Host</label>
                <input
                  type="text"
                  className="form-input"
                  value={settings.server.host}
                  onChange={(e) => updateServer({ host: e.target.value })}
                  placeholder="127.0.0.1"
                />
              </div>
              <div className="form-group">
                <label className="form-label">Server Port</label>
                <input
                  type="number"
                  className="form-input"
                  value={settings.server.port}
                  onChange={(e) => updateServer({ port: parseInt(e.target.value) || 8192 })}
                />
              </div>
              <div className="form-group">
                <label className="form-label">Store Path</label>
                <input
                  type="text"
                  className="form-input"
                  value={settings.server.store_path}
                  onChange={(e) => updateServer({ store_path: e.target.value })}
                  placeholder="./memst-store"
                />
              </div>
              <div className="form-group">
                <label className="form-label">API Base URL</label>
                <input
                  type="text"
                  className="form-input"
                  value={`http://${settings.server.host}:${settings.server.port}/api/v1`}
                  disabled
                  readOnly
                  style={{ background: 'var(--bg-main)' }}
                />
              </div>
            </div>
          )}

          {activeTab === 'kg-extraction' && (
            <div>
              <div className="form-group">
                <label className="form-label" style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                  <input
                    type="checkbox"
                    checked={settings.kg_extraction.enabled}
                    onChange={(e) => updateKGExtraction({ enabled: e.target.checked })}
                  />
                  Enable KG Extraction v2
                </label>
                <p style={{ fontSize: '12px', color: 'var(--text-muted)', marginTop: '4px' }}>
                  Enable entity extraction using the KG Extraction v2 engine
                </p>
              </div>
              <div className="form-group">
                <label className="form-label">Database Path (optional)</label>
                <input
                  type="text"
                  className="form-input"
                  value={settings.kg_extraction.db_path || ''}
                  onChange={(e) => updateKGExtraction({ db_path: e.target.value || null })}
                  placeholder="Leave empty for in-memory storage"
                />
                <p style={{ fontSize: '12px', color: 'var(--text-muted)', marginTop: '4px' }}>
                  Path to SQLite database file. Leave empty to use in-memory storage.
                </p>
              </div>
              <div className="form-group">
                <label className="form-label">Default Ontology ID (optional)</label>
                <input
                  type="text"
                  className="form-input"
                  value={settings.kg_extraction.default_ontology || ''}
                  onChange={(e) => updateKGExtraction({ default_ontology: e.target.value || null })}
                  placeholder="e.g., lingyuqingbao-lei-kejiqingbao-rengongzhineng"
                />
                <p style={{ fontSize: '12px', color: 'var(--text-muted)', marginTop: '4px' }}>
                  Default ontology to use for extraction when none specified
                </p>
              </div>
            </div>
          )}
        </div>
        <div className="modal-footer">
          <button className="btn btn-secondary" onClick={handleReset} disabled={isSaving}>
            Reset to Default
          </button>
          <button className="btn btn-primary" onClick={handleSave} disabled={isSaving}>
            {isSaving ? (
              <>
                <i className="fas fa-spinner fa-spin" style={{ marginRight: '6px' }}></i>
                Saving...
              </>
            ) : (
              'Save & Close'
            )}
          </button>
        </div>
      </div>
    </div>
  );
}
