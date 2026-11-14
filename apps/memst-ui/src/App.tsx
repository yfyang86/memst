import React, { useState } from 'react';
import { AppProvider, useApp } from './context/AppContext';
import { SettingsProvider } from './context/SettingsContext';
import { NavigationPanel } from './components/NavigationPanel';
import { Sidebar } from './components/Sidebar';
import { ChatArea } from './components/ChatArea';
import { CanvasPanel } from './components/CanvasPanel';
import { SearchPanel } from './components/SearchPanel';
import { SettingsPanel } from './components/SettingsPanel';
import { useSettings as useSettingsData } from './context/SettingsContext';
import './styles/index.css';

function truncateModel(model: string): string {
  if (!model) return '';
  if (model.length <= 20) return model;
  // Extract just the model name from path
  const parts = model.split('/');
  const name = parts[parts.length - 1] || parts[parts.length - 2] || model;
  if (name.length <= 20) return name;
  return name.substring(0, 18) + '...';
}

function AppContent() {
  const { state, loadSession } = useApp();
  const { settings } = useSettingsData();
  const [showSettings, setShowSettings] = useState(false);
  const [viewMode, setViewMode] = useState<'chat' | 'split'>('chat');
  const [highlightedMessage, setHighlightedMessage] = useState<string | null>(null);

  // Update API base URL when server settings change
  React.useEffect(() => {
    const baseUrl = `http://${settings.server.host}:${settings.server.port}/api/v1`;
    // In a real app, you'd update the API client here
    console.log('API Base URL:', baseUrl);
  }, [settings.server.host, settings.server.port]);

  const handleHighlightMessage = async (messageId: string, sessionId: string) => {
    // Switch to split view
    setViewMode('split');
    // Load the session if different from current
    if (state.currentSession?.id !== sessionId) {
      await loadSession(sessionId);
    }
    // Highlight the message
    setHighlightedMessage(messageId);
    // Clear highlight after animation
    setTimeout(() => setHighlightedMessage(null), 3000);
  };

  return (
    <div className="app-container">
      <NavigationPanel onSettingsClick={() => setShowSettings(true)} />
      <Sidebar />
      <main className="main-area">
        <header className="main-header">
          <div className="session-title">
            <h1>{state.currentSession?.name || 'No Session Selected'}</h1>
            {state.currentSession && (
              <>
                <span className="session-badge">{state.currentSession.session_type}</span>
                <span className="session-model" title={state.currentSession.model}>
                  <i className="fas fa-microchip"></i>
                  {truncateModel(state.currentSession.model)}
                </span>
              </>
            )}
          </div>
          <div className="view-toggle">
            <button
              className={viewMode === 'chat' ? 'active' : ''}
              onClick={() => setViewMode('chat')}
            >
              <i className="fas fa-comment"></i> Chat
            </button>
            <button
              className={viewMode === 'split' ? 'active' : ''}
              onClick={() => setViewMode('split')}
            >
              <i className="fas fa-columns"></i> Split
            </button>
          </div>
          <div className="header-actions">
            <button className="header-btn" title="Search in session">
              <i className="fas fa-search"></i>
            </button>
            <button className="header-btn" title="Branch session">
              <i className="fas fa-code-branch"></i>
            </button>
            <button className="header-btn" title="Export">
              <i className="fas fa-download"></i>
            </button>
            <button className="header-btn" title="Settings" onClick={() => setShowSettings(true)}>
              <i className="fas fa-cog"></i>
            </button>
          </div>
        </header>
        <div className={`main-content ${viewMode}`}>
          {viewMode === 'chat' ? (
            <>
              <ChatArea highlightedMessage={highlightedMessage} />
              <CanvasPanel />
            </>
          ) : (
            <>
              <ChatArea highlightedMessage={highlightedMessage} />
              <SearchPanel onMessageClick={handleHighlightMessage} />
            </>
          )}
        </div>
      </main>

      <SettingsPanel isOpen={showSettings} onClose={() => setShowSettings(false)} />
    </div>
  );
}

export default function App() {
  return (
    <SettingsProvider>
      <AppProvider>
        <AppContent />
      </AppProvider>
    </SettingsProvider>
  );
}
