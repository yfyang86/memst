import React, { useState, useRef, useEffect } from 'react';
import { useApp } from '../context/AppContext';
import { Message } from '../types';
import { memstApi } from '../api/client';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

interface MessageItemProps {
  message: Message;
  isStreaming?: boolean;
  renderMarkdown?: boolean;
  highlighted?: boolean;
  messageRef?: (el: HTMLDivElement | null) => void;
}

function MessageItem({ message, isStreaming, renderMarkdown, highlighted, messageRef }: MessageItemProps) {
  const roleIcons: Record<string, string> = {
    user: '',
    assistant: 'fa-robot',
    system: 'fa-info',
  };

  const roleLabels: Record<string, string> = {
    user: '',
    assistant: '',
    system: 'System',
  };

  const userInitials = message.role === 'user' ? 'JD' : '';

  const formatTime = (isoString: string): string => {
    if (!isoString) return '';
    const date = new Date(isoString);
    return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  };

  const [copied, setCopied] = React.useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(message.content);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('Failed to copy:', err);
    }
  };

  return (
    <div
      className={`message ${message.role}${highlighted ? ' highlighted' : ''}`}
      data-message-id={message.id}
      ref={messageRef}
    >
      <div className="message-avatar">
        {message.role === 'user' ? (
          userInitials
        ) : (
          <i className={`fas ${roleIcons[message.role] || 'fa-robot'}`}></i>
        )}
      </div>
      <div className="message-content">
        <div className="message-text">
          {renderMarkdown ? (
            <ReactMarkdown remarkPlugins={[remarkGfm]}>{message.content}</ReactMarkdown>
          ) : (
            <div dangerouslySetInnerHTML={{ __html: formatContent(message.content) }} />
          )}
        </div>
        {isStreaming && (
          <div className="streaming-indicator">
            <span className="typing-cursor"></span>
          </div>
        )}
        <div className="message-meta">
          {roleLabels[message.role] && <span>{roleLabels[message.role]}</span>}
          {message.timestamp && (
            <>
              <span>•</span>
              <span>{formatTime(message.timestamp)}</span>
            </>
          )}
          {'latency' in (message.metadata || {}) && (
            <>
              <span>•</span>
              <span>{String(message.metadata?.latency as number | string | undefined)} latency</span>
            </>
          )}
        </div>
        {message.agent_trace && message.agent_trace.length > 0 && (
          <div className="agent-trace">
            <div className="agent-trace-title">
              <i className="fas fa-sitemap"></i>
              Agent Trace
            </div>
            {message.agent_trace.map((step) => (
              <div key={step.id} className={`agent-step ${step.status}`}>
                <i
                  className={`fas ${
                    step.status === 'success'
                      ? 'fa-check-circle'
                      : step.status === 'pending'
                      ? 'fa-clock'
                      : 'fa-exclamation-circle'
                  }`}
                ></i>
                <span>
                  {step.title}: {step.detail}
                </span>
              </div>
            ))}
          </div>
        )}
        {message.role !== 'system' && (
          <div className="message-actions">
            <button onClick={handleCopy} title={copied ? 'Copied!' : 'Copy to clipboard'}>
              <i className={`fas ${copied ? 'fa-check' : 'fa-copy'}`}></i>
              {copied ? 'Copied!' : 'Copy'}
            </button>
            <button className="action-disabled" title="Coming soon">
              <i className="fas fa-brain"></i> Save to Memory
            </button>
            {message.role === 'assistant' && (
              <button className="action-disabled" title="Coming soon">
                <i className="fas fa-redo"></i> Regenerate
              </button>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

function formatContent(content: string): string {
  if (!content) return '';
  // Simple markdown-like formatting
  return content
    .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
    .replace(/\*(.*?)\*/g, '<em>$1</em>')
    .replace(/\n/g, '<br>');
}

interface ThinkingIndicatorProps {
  visible: boolean;
}

function ThinkingIndicator({ visible }: ThinkingIndicatorProps) {
  if (!visible) return null;

  return (
    <div className="message assistant">
      <div className="message-avatar">
        <i className="fas fa-robot"></i>
      </div>
      <div className="message-content">
        <div className="typing-indicator">
          <div className="typing-dots">
            <span></span>
            <span></span>
            <span></span>
          </div>
          <span>Thinking...</span>
        </div>
      </div>
    </div>
  );
}

interface ChatAreaProps {
  highlightedMessage?: string | null;
}

export function ChatArea({ highlightedMessage }: ChatAreaProps) {
  const { state, dispatch } = useApp();
  const [inputValue, setInputValue] = useState('');
  const [isTyping, setIsTyping] = useState(false);
  const [attachments, setAttachments] = useState<File[]>([]);
  const [streaming, setStreaming] = useState(false);
  const [renderMarkdown, setRenderMarkdown] = useState(true);
  const [streamingContent, setStreamingContent] = useState('');
  const [isStreamingMessage, setIsStreamingMessage] = useState(false);
  const [isReloadingMemory, setIsReloadingMemory] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  // Refs map for message elements to scroll to highlighted message
  const messageRefs = useRef<Map<string, HTMLDivElement>>(new Map());

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  };

  // Scroll to highlighted message when it changes
  useEffect(() => {
    if (highlightedMessage) {
      const messageEl = messageRefs.current.get(highlightedMessage);
      if (messageEl) {
        messageEl.scrollIntoView({ behavior: 'smooth', block: 'center' });
      }
    }
  }, [highlightedMessage]);

  useEffect(() => {
    scrollToBottom();
  }, [state.messages, isTyping, streamingContent]);

  // Handle streaming chat
  const handleStreamingChat = async (content: string) => {
    if (!state.currentSession) return;

    // Add user message immediately (optimistic update)
    const userMessage: Message = {
      id: `user-${Date.now()}`,
      session_id: state.currentSession.id,
      role: 'user',
      content: content,
      timestamp: new Date().toISOString(),
      attachments: [],
      metadata: {},
    };
    dispatch({ type: 'ADD_MESSAGE', payload: userMessage });
    setInputValue('');
    setIsStreamingMessage(true);
    setStreamingContent('');

    try {
      let assistantContent = '';

      // Use agent or regular chat endpoint based on session type
      const generator = state.currentSession.session_type === 'agent'
        ? memstApi.agentChatStream(state.currentSession.id, content, state.currentSession.model)
        : memstApi.chatStream(state.currentSession.id, content, state.currentSession.model);

      for await (const chunk of generator) {
        assistantContent += chunk;
        setStreamingContent(assistantContent);
      }

      // Create complete assistant message
      const assistantMessage: Message = {
        id: `asst-${Date.now()}`,
        session_id: state.currentSession.id,
        role: 'assistant',
        content: assistantContent,
        timestamp: new Date().toISOString(),
        attachments: [],
        metadata: { model: state.currentSession.model, agent: state.currentSession.session_type === 'agent' },
      };

      dispatch({ type: 'ADD_MESSAGE', payload: assistantMessage });

      // Reload messages to ensure consistency - use correct endpoint based on session type
      const isAgent = state.currentSession.session_type === 'agent';
      const messages = isAgent
        ? await memstApi.getAgentHistory(state.currentSession.id)
        : await memstApi.getMessages(state.currentSession.id);
      dispatch({ type: 'SET_MESSAGES', payload: messages as Message[] });
    } catch (err) {
      console.error('Streaming error:', err);
      dispatch({ type: 'SET_ERROR', payload: 'Failed to get streaming response' });
    } finally {
      setIsStreamingMessage(false);
      setStreamingContent('');
      setIsTyping(false);
    }
  };

  const handleSend = async () => {
    if (!inputValue.trim() && attachments.length === 0) return;

    const content = inputValue;
    setInputValue('');

    if (streaming) {
      await handleStreamingChat(content);
    } else {
      // Non-streaming chat: show user message immediately with thinking animation
      const userMessage: Message = {
        id: `user-${Date.now()}`,
        session_id: state.currentSession!.id,
        role: 'user',
        content: content,
        timestamp: new Date().toISOString(),
        attachments: [],
        metadata: {},
      };
      dispatch({ type: 'ADD_MESSAGE', payload: userMessage });
      setIsTyping(true);

      try {
        // Use agent or regular chat endpoint based on session type
        const isAgent = state.currentSession!.session_type === 'agent';
        const assistantMessage = isAgent
          ? await memstApi.agentChat(state.currentSession!.id, content, state.currentSession!.model)
          : await memstApi.chat(state.currentSession!.id, content, state.currentSession!.model);

        // Add assistant message
        dispatch({ type: 'ADD_MESSAGE', payload: assistantMessage as Message });

        // Reload messages - use correct endpoint based on session type
        const messages = isAgent
          ? await memstApi.getAgentHistory(state.currentSession!.id)
          : await memstApi.getMessages(state.currentSession!.id);
        dispatch({ type: 'SET_MESSAGES', payload: messages as Message[] });
      } catch (err) {
        dispatch({ type: 'SET_ERROR', payload: 'Failed to send message' });
      } finally {
        setIsTyping(false);
      }
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      return;
    }
    if (e.key === 'Enter' && e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = Array.from(e.target.files || []);
    setAttachments(prev => [...prev, ...files]);
    if (fileInputRef.current) {
      fileInputRef.current.value = '';
    }
  };

  const removeAttachment = (index: number) => {
    setAttachments(prev => prev.filter((_, i) => i !== index));
  };

  const triggerFileSelect = () => {
    fileInputRef.current?.click();
  };

  const handleReloadMemory = async () => {
    if (!state.currentSession || isReloadingMemory) return;

    setIsReloadingMemory(true);
    try {
      const result = await memstApi.reloadMemory(state.currentSession.id);
      if (result.success) {
        // Show info message - using SET_ERROR with null to clear any existing error
        dispatch({ type: 'SET_ERROR', payload: null });
        console.log(`Reloaded ${result.memories_added} memories from messages`);
        // Reload memories to update the display
        const memories = await memstApi.getAllMemories(state.currentSession.id);
        // Cast to the correct type
        dispatch({
          type: 'SET_MEMORIES',
          payload: memories as { working: typeof state.memories.working; short: typeof state.memories.short; long: typeof state.memories.long },
        });
      } else {
        dispatch({ type: 'SET_ERROR', payload: result.message || 'Failed to reload memory' });
      }
    } catch (err) {
      console.error('Reload memory error:', err);
      dispatch({ type: 'SET_ERROR', payload: 'Failed to reload memory' });
    } finally {
      setIsReloadingMemory(false);
    }
  };

  return (
    <div className="chat-area">
      <div className="messages-container">
        {state.messages.length === 0 && !isStreamingMessage ? (
          <div className="empty-state">
            <i className="fas fa-comments"></i>
            <h3>No messages yet</h3>
            <p>Start a conversation with your assistant</p>
          </div>
        ) : (
          <>
            {state.messages.map((message) => (
              <MessageItem
                key={message.id}
                message={message}
                renderMarkdown={renderMarkdown}
                highlighted={message.id === highlightedMessage}
                messageRef={(el) => {
                  if (el) {
                    messageRefs.current.set(message.id, el);
                  } else {
                    messageRefs.current.delete(message.id);
                  }
                }}
              />
            ))}
            {isStreamingMessage && (
              <div className="message assistant">
                <div className="message-avatar">
                  <i className="fas fa-robot"></i>
                </div>
                <div className="message-content">
                  <div className="message-text">
                    {renderMarkdown ? (
                      <ReactMarkdown remarkPlugins={[remarkGfm]}>{streamingContent}</ReactMarkdown>
                    ) : (
                      <div dangerouslySetInnerHTML={{ __html: formatContent(streamingContent) }} />
                    )}
                  </div>
                  <div className="streaming-indicator">
                    <span className="typing-cursor"></span>
                  </div>
                </div>
              </div>
            )}
            <ThinkingIndicator visible={isTyping && !isStreamingMessage} />
          </>
        )}
        <div ref={messagesEndRef} />
      </div>

      <div className="input-area">
        {attachments.length > 0 && (
          <div className="input-attachments">
            {attachments.map((file, index) => (
              <div key={index} className="attachment-chip">
                <i className="fas fa-file-code"></i>
                <span>{file.name}</span>
                <span className="remove" onClick={() => removeAttachment(index)}>
                  <i className="fas fa-times"></i>
                </span>
              </div>
            ))}
          </div>
        )}
        <div className="input-header">
          <div className="input-tools">
            <input
              type="file"
              ref={fileInputRef}
              style={{ display: 'none' }}
              onChange={handleFileSelect}
              multiple
            />
            <button title="Attach file" onClick={triggerFileSelect}>
              <i className="fas fa-paperclip"></i>
            </button>
            <button
              title="Reload memory from messages"
              onClick={handleReloadMemory}
              disabled={isReloadingMemory || !state.currentSession}
            >
              <i className={`fas fa-sync-alt ${isReloadingMemory ? 'fa-spin' : ''}`}></i>
            </button>
            <button title="Code snippet">
              <i className="fas fa-code"></i>
            </button>
            <button title="Voice input">
              <i className="fas fa-microphone"></i>
            </button>
          </div>
          <div className="input-actions">
            <span className="toggle-group">
              <label className="toggle-switch" title="Markdown">
                <input type="checkbox" checked={renderMarkdown} onChange={(e) => setRenderMarkdown(e.target.checked)} />
                <span className="toggle-slider"></span>
              </label>
              <span title="Markdown">
                <i className="fab fa-markdown"></i>
              </span>
            </span>
            <span className="toggle-group">
              <label className="toggle-switch" title="Stream response">
                <input type="checkbox" checked={streaming} onChange={(e) => setStreaming(e.target.checked)} />
                <span className="toggle-slider"></span>
              </label>
              <span title="Stream">
                <i className="fas fa-stream"></i>
              </span>
            </span>
          </div>
        </div>
        <div className="input-wrapper">
          <textarea
            className="input-field"
            placeholder="Type a message... (Shift+Enter to send)"
            rows={3}
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            onKeyDown={handleKeyDown}
          />
          <button className="send-btn" onClick={handleSend} disabled={!inputValue.trim() && attachments.length === 0}>
            <i className="fas fa-paper-plane"></i>
          </button>
        </div>
        <div className="input-hints">
          <span>Enter = new line, Shift+Enter = send</span>
          <span>
            {state.currentSession?.session_type === 'agent' ? (
              <><i className="fas fa-robot"></i> Agent: {state.currentSession?.model?.split('/').pop() || 'gpt-4'}</>
            ) : (
              <>Model: {state.currentSession?.model?.split('/').pop() || 'gpt-4'}</>
            )}
          </span>
        </div>
      </div>
    </div>
  );
}
