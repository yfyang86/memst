import React, { useState } from 'react';
import { memstApi } from '../api/client';

interface SearchResult {
  id: string;
  session_id: string;
  session_name: string;
  role: string;
  content: string;
  timestamp: string;
  score: number;
  highlight?: string;
  doc_type?: string;
}

interface SearchPanelProps {
  onMessageClick: (messageId: string, sessionId: string) => void;
}

type SearchType = 'text' | 'semantic' | 'regex' | 'hybrid';

export function SearchPanel({ onMessageClick }: SearchPanelProps) {
  const [query, setQuery] = useState('');
  const [searchType, setSearchType] = useState<SearchType>('hybrid');
  const [results, setResults] = useState<SearchResult[]>([]);
  const [loading, setLoading] = useState(false);
  const [hasSearched, setHasSearched] = useState(false);

  const handleSearch = async () => {
    if (!query.trim()) return;

    setLoading(true);
    setHasSearched(true);

    try {
      const searchResults = await memstApi.search(query, searchType);
      // Transform results to include highlights and session info
      const transformedResults: SearchResult[] = (searchResults as any[]).map((item) => ({
        id: item.id || item.message_id || `result-${Math.random()}`,
        session_id: item.session_id || '',
        session_name: item.session_name || '',
        role: item.role || 'assistant',
        content: item.content || item.text || '',
        timestamp: item.timestamp || '',
        score: item.score || 0.8,
        highlight: item.highlight || generateSnippet(item.content || item.text || '', query),
        doc_type: item.doc_type || 'message',
      }));
      setResults(transformedResults);
    } catch (err) {
      console.error('Search failed:', err);
      setResults([]);
    } finally {
      setLoading(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSearch();
    }
  };

  const formatTime = (isoString: string): string => {
    if (!isoString) return '';
    const date = new Date(isoString);
    return date.toLocaleDateString() + ' ' + date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  };

  return (
    <div className="search-panel">
      <div className="search-header">
        <h3><i className="fas fa-search"></i> Search Session</h3>
      </div>

      <div className="search-input-section">
        <div className="search-type-tabs">
          {[
            { key: 'text', label: 'Full Text', icon: 'fa-align-left' },
            { key: 'semantic', label: 'Semantic', icon: 'fa-brain' },
            { key: 'regex', label: 'Regex', icon: 'fa-code' },
            { key: 'hybrid', label: 'Hybrid', icon: 'fa-magic' },
          ].map((type) => (
            <button
              key={type.key}
              className={`search-type-tab ${searchType === type.key ? 'active' : ''}`}
              onClick={() => setSearchType(type.key as SearchType)}
              title={type.label}
            >
              <i className={`fas ${type.icon}`}></i>
              <span>{type.label}</span>
            </button>
          ))}
        </div>

        <div className="search-input-wrapper">
          <input
            type="text"
            className="search-input"
            placeholder={searchType === 'regex' ? 'Enter regex pattern...' : 'Search conversations...'}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={handleKeyDown}
          />
          <button className="search-btn" onClick={handleSearch} disabled={loading}>
            <i className={`fas ${loading ? 'fa-spinner fa-spin' : 'fa-search'}`}></i>
          </button>
        </div>

        {searchType === 'regex' && (
          <div className="search-hint">
            <i className="fas fa-info-circle"></i>
            Use regex patterns like <code>error|warn</code>, <code>^User:</code>, <code>\d+</code>
          </div>
        )}
      </div>

      <div className="search-results">
        {!hasSearched ? (
          <div className="search-empty">
            <i className="fas fa-search"></i>
            <p>Search through your conversation history</p>
            <div className="search-tips">
              <h4>Search Types:</h4>
              <ul>
                <li><strong>Full Text:</strong> Exact keyword matching</li>
                <li><strong>Semantic:</strong> Meaning-based search using embeddings</li>
                <li><strong>Regex:</strong> Pattern-based search</li>
                <li><strong>Hybrid:</strong> Combines text + semantic ranking</li>
              </ul>
            </div>
          </div>
        ) : loading ? (
          <div className="search-loading">
            <i className="fas fa-spinner fa-spin"></i>
            <span>Searching...</span>
          </div>
        ) : results.length === 0 ? (
          <div className="search-empty">
            <i className="fas fa-search"></i>
            <p>No results found for "{query}"</p>
            <p className="search-suggestion">Try different keywords or search type</p>
          </div>
        ) : (
          <>
            <div className="search-results-header">
              <span>{results.length} result{results.length !== 1 ? 's' : ''}</span>
            </div>
            {results.map((result) => (
              <div
                key={result.id}
                className="search-result-item"
                onClick={() => onMessageClick(result.id, result.session_id)}
              >
                <div className="result-header">
                  <span className={`result-role ${result.role}`}>
                    {result.role === 'user' ? (
                      <>
                        <i className="fas fa-user"></i> You
                      </>
                    ) : (
                      <>
                        <i className="fas fa-robot"></i> Assistant
                      </>
                    )}
                  </span>
                  {result.session_name && (
                    <span className="result-session">
                      <i className="fas fa-comments"></i>
                      {result.session_name}
                    </span>
                  )}
                  <span className="result-time">{formatTime(result.timestamp)}</span>
                </div>
                <div
                  className="result-content"
                  dangerouslySetInnerHTML={{ __html: result.highlight || result.content }}
                />
                <div className="result-footer">
                  <span className="result-score">
                    <i className="fas fa-bullseye"></i>
                    {Math.round(result.score * 100)}% match
                  </span>
                  <span className="result-action">
                    <i className="fas fa-location-arrow"></i>
                    Go to message
                  </span>
                </div>
              </div>
            ))}
          </>
        )}
      </div>
    </div>
  );
}

function generateSnippet(content: string, query: string, maxLength = 200): string {
  if (!query.trim()) {
    return content.slice(0, maxLength) + (content.length > maxLength ? '...' : '');
  }

  const lowerContent = content.toLowerCase();
  const lowerQuery = query.toLowerCase();
  const index = lowerContent.indexOf(lowerQuery);

  if (index === -1) {
    return content.slice(0, maxLength) + (content.length > maxLength ? '...' : '');
  }

  const start = Math.max(0, index - 50);
  const end = Math.min(content.length, index + query.length + 150);
  let snippet = content.slice(start, end);

  if (start > 0) snippet = '...' + snippet;
  if (end < content.length) snippet = snippet + '...';

  // Highlight the matched term
  const regex = new RegExp(`(${escapeRegex(query)})`, 'gi');
  snippet = snippet.replace(regex, '<mark>$1</mark>');

  return snippet;
}

function escapeRegex(str: string): string {
  return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}
