import { useState, useEffect } from 'react';
import { useApp } from '../context/AppContext';
import { useSettings } from '../context/SettingsContext';
import { Session, File as FileType } from '../types';

interface SessionItemProps {
  session: Session;
  active: boolean;
  onClick: () => void;
  onDelete: () => void;
}

function SessionItem({ session, active, onClick, onDelete }: SessionItemProps) {
  const typeIcons: Record<string, string> = {
    chat: 'fa-comment',
    task: 'fa-tasks',
    search: 'fa-search',
    recommend: 'fa-lightbulb',
    agent: 'fa-robot',
  };

  return (
    <div className={`session-item ${active ? 'active' : ''}`} onClick={onClick}>
      <div className={`session-icon ${session.session_type}`}>
        <i className={`fas ${typeIcons[session.session_type] || 'fa-comment'}`}></i>
      </div>
      <div className="session-info">
        <div className="session-name">{session.name}</div>
        <div className="session-meta">
          {session.session_type === 'agent' && (
            <span className="agent-badge">
              <i className="fas fa-robot"></i> Agent
            </span>
          )}
          <span>
            <i className="fas fa-clock"></i> {formatTime(session.updated_at)}
          </span>
          <span>{session.message_count} msgs</span>
        </div>
      </div>
      <div className="session-actions">
        <button onClick={(e) => { e.stopPropagation(); onDelete(); }}>
          <i className="fas fa-ellipsis-v"></i>
        </button>
      </div>
    </div>
  );
}

function formatTime(isoString: string): string {
  if (!isoString) return 'now';
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

function formatDateHeader(isoString: string): string {
  if (!isoString) return '';
  const date = new Date(isoString);
  const now = new Date();
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const yesterday = new Date(today);
  yesterday.setDate(yesterday.getDate() - 1);
  const thisWeekStart = new Date(today);
  thisWeekStart.setDate(thisWeekStart.getDate() - thisWeekStart.getDay());
  const thisMonthStart = new Date(now.getFullYear(), now.getMonth(), 1);

  const msgDate = new Date(date.getFullYear(), date.getMonth(), date.getDate());

  if (msgDate >= today) return 'Today';
  if (msgDate >= yesterday) return 'Yesterday';
  if (msgDate >= thisWeekStart) return 'This Week';
  if (msgDate >= thisMonthStart) return 'This Month';
  return 'Earlier';
}

interface ResourceItemProps {
  file: FileType;
}

function ResourceItem({ file }: ResourceItemProps) {
  const typeIcons: Record<string, string> = {
    pdf: 'fa-file-pdf',
    doc: 'fa-file-alt',
    img: 'fa-file-image',
    code: 'fa-file-code',
  };

  return (
    <div className="resource-item">
      <div className={`resource-icon ${file.type}`}>
        <i className={`fas ${typeIcons[file.type] || 'fa-file'}`}></i>
      </div>
      <div className="resource-name">{file.name}</div>
      <div className="resource-size">{formatSize(file.size)}</div>
    </div>
  );
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${Math.round(bytes / (1024 * 1024))} MB`;
}

interface NewSessionModalProps {
  isOpen: boolean;
  onClose: () => void;
  onCreate: (name: string, type: string, model: string) => void;
  defaultModel?: string;
}

function NewSessionModal({ isOpen, onClose, onCreate, defaultModel }: NewSessionModalProps) {
  const { settings } = useSettings();
  const [name, setName] = useState('');
  const [type, setType] = useState('chat');
  const [model, setModel] = useState(defaultModel || settings.llm.model || 'gpt-4');
  const [isCustomModel, setIsCustomModel] = useState(false);
  const [customModelInput, setCustomModelInput] = useState(model);

  // Update model when defaultModel changes
  useEffect(() => {
    if (defaultModel) {
      setModel(defaultModel);
      setCustomModelInput(defaultModel);
    }
  }, [defaultModel]);

  // Check if the current model matches settings to determine if custom
  useEffect(() => {
    if (settings.llm.model && model !== settings.llm.model) {
      setIsCustomModel(true);
      setCustomModelInput(model);
    }
  }, [model, settings.llm.model]);

  const handleModelChange = (value: string) => {
    if (value === '__custom__') {
      setIsCustomModel(true);
      setModel(customModelInput);
    } else {
      setIsCustomModel(false);
      setModel(value);
    }
  };

  const handleCustomModelInput = (value: string) => {
    setCustomModelInput(value);
    setModel(value);
  };

  if (!isOpen) return null;

  const handleSubmit = () => {
    if (name.trim()) {
      onCreate(name.trim(), type, model);
      setName('');
      onClose();
    }
  };

  const typeOptions = [
    { id: 'chat', icon: 'fa-comment', label: 'Chat' },
    { id: 'agent', icon: 'fa-robot', label: 'Agent' },
    { id: 'search', icon: 'fa-search', label: 'Search' },
    { id: 'task', icon: 'fa-tasks', label: 'Task' },
    { id: 'recommend', icon: 'fa-lightbulb', label: 'Recommend' },
  ];

  return (
    <div className={`modal-overlay ${isOpen ? 'active' : ''}`} onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>Create New Session</h2>
          <button className="modal-close" onClick={onClose}>
            <i className="fas fa-times"></i>
          </button>
        </div>
        <div className="modal-body">
          <div className="form-group">
            <label className="form-label">Session Name</label>
            <input
              type="text"
              className="form-input"
              placeholder="e.g., Rust Project Help"
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </div>
          <div className="form-group">
            <label className="form-label">Session Type</label>
            <div className="session-type-options">
              {typeOptions.map((opt) => (
                <div
                  key={opt.id}
                  className={`session-type-option ${type === opt.id ? 'selected' : ''}`}
                  onClick={() => setType(opt.id)}
                >
                  <i className={`fas ${opt.icon}`}></i>
                  <span>{opt.label}</span>
                </div>
              ))}
            </div>
          </div>
          <div className="form-group">
            <label className="form-label">Model</label>
            {isCustomModel ? (
              <input
                type="text"
                className="form-input"
                value={customModelInput}
                onChange={(e) => handleCustomModelInput(e.target.value)}
                placeholder="Enter model name or path"
              />
            ) : (
              <select
                className="form-select"
                value={model}
                onChange={(e) => handleModelChange(e.target.value)}
              >
                <option value={settings.llm.model}>
                  {settings.llm.model} (Default)
                </option>
                <option value="gpt-4">GPT-4</option>
                <option value="gpt-4-turbo">GPT-4 Turbo</option>
                <option value="claude-3">Claude 3</option>
                <option value="claude-3-opus">Claude 3 Opus</option>
                <option value="__custom__">Custom model...</option>
              </select>
            )}
            {!isCustomModel && model !== settings.llm.model && (
              <button
                className="btn btn-secondary"
                style={{ marginTop: '8px', fontSize: '12px', padding: '6px 12px' }}
                onClick={() => handleModelChange('__custom__')}
              >
                <i className="fas fa-pencil-alt" style={{ marginRight: '4px' }}></i>
                Use custom model
              </button>
            )}
            {isCustomModel && (
              <button
                className="btn btn-secondary"
                style={{ marginTop: '8px', fontSize: '12px', padding: '6px 12px' }}
                onClick={() => {
                  setIsCustomModel(false);
                  setModel(settings.llm.model);
                }}
              >
                <i className="fas fa-undo" style={{ marginRight: '4px' }}></i>
                Use default ({settings.llm.model})
              </button>
            )}
          </div>
        </div>
        <div className="modal-footer">
          <button className="btn btn-secondary" onClick={onClose}>
            Cancel
          </button>
          <button className="btn btn-primary" onClick={handleSubmit}>
            Create Session
          </button>
        </div>
      </div>
    </div>
  );
}

export function Sidebar() {
  const { state, dispatch, createSession, deleteSession, loadSession } = useApp();
  const { settings } = useSettings();
  const [showModal, setShowModal] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');

  const user = state.user;
  const userInitials = user?.name?.split(' ').map((n) => n[0]).join('').toUpperCase() || 'U';

  const handleSessionClick = (session: Session) => {
    loadSession(session.id);
  };

  const handleCreateSession = (name: string, type: string, model: string) => {
    createSession(name, type, model);
  };

  const handleDeleteSession = (id: string) => {
    if (confirm('Delete this session?')) {
      deleteSession(id);
    }
  };

  const filteredSessions = state.sessions.filter((s) =>
    s.name.toLowerCase().includes(searchQuery.toLowerCase())
  );

  return (
    <aside className="sidebar">
      <div className="sidebar-header">
        <div className="user-section">
          <div className="user-avatar">{userInitials}</div>
          <div className="user-info">
            <div className="user-name">{user?.name || 'User'}</div>
            <div className="user-role">Developer</div>
          </div>
          <div className="user-dropdown">
            <i className="fas fa-chevron-down"></i>
          </div>
        </div>
        <div className="search-box">
          <i className="fas fa-search"></i>
          <input
            type="text"
            placeholder="Search sessions, messages..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
          />
          <kbd>⌘K</kbd>
        </div>
      </div>

      <div className="sidebar-tabs">
        {['sessions', 'resources', 'history'].map((tab) => (
          <div
            key={tab}
            className={`sidebar-tab ${state.activeSidebarTab === tab ? 'active' : ''}`}
            onClick={() => dispatch({ type: 'SET_ACTIVE_SIDEBAR_TAB', payload: tab })}
          >
            {tab.charAt(0).toUpperCase() + tab.slice(1)}
          </div>
        ))}
      </div>

      <div className="sidebar-content">
        {state.activeSidebarTab === 'sessions' && (
          <>
            <div className="section-title">
              <span>Active Sessions</span>
              <button onClick={() => setShowModal(true)}>
                <i className="fas fa-plus"></i>
              </button>
            </div>
            <div className="session-list">
              {filteredSessions.length === 0 ? (
                <div className="empty-state">
                  <i className="fas fa-comments"></i>
                  <h3>No sessions</h3>
                  <p>Create a new session to get started</p>
                </div>
              ) : (
                filteredSessions.map((session) => (
                  <SessionItem
                    key={session.id}
                    session={session}
                    active={state.currentSession?.id === session.id}
                    onClick={() => handleSessionClick(session)}
                    onDelete={() => handleDeleteSession(session.id)}
                  />
                ))
              )}
            </div>
          </>
        )}

        {state.activeSidebarTab === 'resources' && (
          <>
            <div className="section-title">
              <span>Recent Files</span>
              <button>
                <i className="fas fa-upload"></i>
              </button>
            </div>
            <div className="resource-list">
              {state.files.map((file) => (
                <ResourceItem key={file.id} file={file} />
              ))}
            </div>
          </>
        )}

        {state.activeSidebarTab === 'history' && (
          <div className="history-list">
            {state.sessions.length === 0 ? (
              <div className="empty-state">
                <i className="fas fa-history"></i>
                <h3>No history</h3>
                <p>Your conversation history will appear here</p>
              </div>
            ) : (
              <>
                {(() => {
                  // Group sessions by date
                  const grouped: Record<string, Session[]> = {};
                  state.sessions.forEach(session => {
                    const header = formatDateHeader(session.updated_at);
                    if (!grouped[header]) grouped[header] = [];
                    grouped[header].push(session);
                  });

                  return Object.entries(grouped).map(([dateHeader, sessions]) => (
                    <div key={dateHeader} className="history-group">
                      <div className="history-group-header">{dateHeader}</div>
                      {sessions.map((session) => (
                        <SessionItem
                          key={session.id}
                          session={session}
                          active={state.currentSession?.id === session.id}
                          onClick={() => handleSessionClick(session)}
                          onDelete={() => handleDeleteSession(session.id)}
                        />
                      ))}
                    </div>
                  ));
                })()}
              </>
            )}
          </div>
        )}
      </div>

      <div className="sidebar-footer">
        <button className="new-session-btn" onClick={() => setShowModal(true)}>
          <i className="fas fa-plus"></i>
          New Session
        </button>
      </div>

      <NewSessionModal
        isOpen={showModal}
        onClose={() => setShowModal(false)}
        onCreate={handleCreateSession}
        defaultModel={settings.llm.model}
      />
    </aside>
  );
}
