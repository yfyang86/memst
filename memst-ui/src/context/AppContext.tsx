import React, { createContext, useContext, useReducer, useEffect, ReactNode } from 'react';
import { User, Session, Message, Memory, KnowledgeGraph, File, Trace } from '../types';
import { memstApi } from '../api/client';

interface AppState {
  user: User | null;
  sessions: Session[];
  currentSession: Session | null;
  messages: Message[];
  memories: { working: Memory[]; short: Memory[]; long: Memory[] };
  knowledgeGraph: KnowledgeGraph | null;
  files: File[];
  traces: Trace[];
  stats: Record<string, unknown>;
  isLoading: boolean;
  error: string | null;
  activeNav: string;
  activeSidebarTab: string;
  activeCanvasTab: string;
  searchQuery: string;
}

type Action =
  | { type: 'SET_USER'; payload: User }
  | { type: 'SET_SESSIONS'; payload: Session[] }
  | { type: 'SET_CURRENT_SESSION'; payload: Session | null }
  | { type: 'ADD_SESSION'; payload: Session }
  | { type: 'DELETE_SESSION'; payload: string }
  | { type: 'SET_MESSAGES'; payload: Message[] }
  | { type: 'ADD_MESSAGE'; payload: Message }
  | { type: 'SET_MEMORIES'; payload: { working: Memory[]; short: Memory[]; long: Memory[] } }
  | { type: 'SET_KNOWLEDGE_GRAPH'; payload: KnowledgeGraph }
  | { type: 'SET_FILES'; payload: File[] }
  | { type: 'SET_TRACES'; payload: Trace[] }
  | { type: 'SET_STATS'; payload: Record<string, unknown> }
  | { type: 'SET_LOADING'; payload: boolean }
  | { type: 'SET_ERROR'; payload: string | null }
  | { type: 'SET_ACTIVE_NAV'; payload: string }
  | { type: 'SET_ACTIVE_SIDEBAR_TAB'; payload: string }
  | { type: 'SET_ACTIVE_CANVAS_TAB'; payload: string }
  | { type: 'SET_SEARCH_QUERY'; payload: string };

const initialState: AppState = {
  user: null,
  sessions: [],
  currentSession: null,
  messages: [],
  memories: { working: [], short: [], long: [] },
  knowledgeGraph: null,
  files: [],
  traces: [],
  stats: {},
  isLoading: false,
  error: null,
  activeNav: 'sessions',
  activeSidebarTab: 'sessions',
  activeCanvasTab: 'memory',
  searchQuery: '',
};

function reducer(state: AppState, action: Action): AppState {
  switch (action.type) {
    case 'SET_USER':
      return { ...state, user: action.payload };
    case 'SET_SESSIONS':
      return { ...state, sessions: action.payload };
    case 'SET_CURRENT_SESSION':
      return { ...state, currentSession: action.payload };
    case 'ADD_SESSION':
      return { ...state, sessions: [action.payload, ...state.sessions] };
    case 'DELETE_SESSION':
      return {
        ...state,
        sessions: state.sessions.filter((s) => s.id !== action.payload),
        currentSession:
          state.currentSession?.id === action.payload
            ? null
            : state.currentSession,
      };
    case 'SET_MESSAGES':
      return { ...state, messages: action.payload };
    case 'ADD_MESSAGE':
      return { ...state, messages: [...state.messages, action.payload] };
    case 'SET_MEMORIES':
      return { ...state, memories: action.payload };
    case 'SET_KNOWLEDGE_GRAPH':
      return { ...state, knowledgeGraph: action.payload };
    case 'SET_FILES':
      return { ...state, files: action.payload };
    case 'SET_TRACES':
      return { ...state, traces: action.payload };
    case 'SET_STATS':
      return { ...state, stats: action.payload };
    case 'SET_LOADING':
      return { ...state, isLoading: action.payload };
    case 'SET_ERROR':
      return { ...state, error: action.payload };
    case 'SET_ACTIVE_NAV':
      return { ...state, activeNav: action.payload };
    case 'SET_ACTIVE_SIDEBAR_TAB':
      return { ...state, activeSidebarTab: action.payload };
    case 'SET_ACTIVE_CANVAS_TAB':
      return { ...state, activeCanvasTab: action.payload };
    case 'SET_SEARCH_QUERY':
      return { ...state, searchQuery: action.payload };
    default:
      return state;
  }
}

interface AppContextType {
  state: AppState;
  dispatch: React.Dispatch<Action>;
  loadSessions: () => Promise<void>;
  loadSession: (id: string) => Promise<void>;
  createSession: (name: string, sessionType: string, model: string) => Promise<void>;
  deleteSession: (id: string) => Promise<void>;
  sendMessage: (content: string) => Promise<void>;
  loadMemories: (sessionId: string) => Promise<void>;
  refreshMemories: (sessionId: string) => Promise<void>;
  parseKnowledgeGraph: (sessionId: string) => Promise<void>;
  search: (query: string) => Promise<void>;
}

const AppContext = createContext<AppContextType | null>(null);

export function AppProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(reducer, initialState);

  // Initialize user
  useEffect(() => {
    memstApi
      .getCurrentUser()
      .then((user) => dispatch({ type: 'SET_USER', payload: user as User }))
      .catch(() => {
        // Create demo user if none exists
        memstApi.login('Demo User', undefined).then((user) => {
          dispatch({ type: 'SET_USER', payload: user as User });
        });
      });
  }, []);

  // Load sessions on mount
  useEffect(() => {
    loadSessions();
  }, []);

  const loadSessions = async () => {
    dispatch({ type: 'SET_LOADING', payload: true });
    try {
      const sessions = await memstApi.listSessions();
      dispatch({ type: 'SET_SESSIONS', payload: sessions as Session[] });
    } catch (err) {
      dispatch({ type: 'SET_ERROR', payload: 'Failed to load sessions' });
    } finally {
      dispatch({ type: 'SET_LOADING', payload: false });
    }
  };

  const loadSession = async (id: string) => {
    dispatch({ type: 'SET_LOADING', payload: true });
    try {
      const session = await memstApi.getSession(id);
      dispatch({ type: 'SET_CURRENT_SESSION', payload: session as Session });

      // Check if this is an agent session
      const isAgent = (session as Session).session_type === 'agent';

      // Load messages from the correct source based on session type
      const messages = isAgent
        ? await memstApi.getAgentHistory(id)
        : await memstApi.getMessages(id);

      const [files, traces, graph, stats] = await Promise.all([
        memstApi.listFiles(id),
        memstApi.getTrace(id),
        memstApi.getKnowledgeGraph(id),
        memstApi.getSessionStats(id),
      ]);

      dispatch({ type: 'SET_MESSAGES', payload: messages as Message[] });
      // Load memories for all session types (including agent sessions)
      const memories = await memstApi.getAllMemories(id);
      dispatch({ type: 'SET_MEMORIES', payload: memories as { working: Memory[]; short: Memory[]; long: Memory[] } });
      dispatch({ type: 'SET_FILES', payload: files as File[] });
      dispatch({ type: 'SET_TRACES', payload: traces as Trace[] });
      dispatch({ type: 'SET_KNOWLEDGE_GRAPH', payload: graph as KnowledgeGraph });
      dispatch({ type: 'SET_STATS', payload: stats as Record<string, unknown> });
    } catch (err) {
      dispatch({ type: 'SET_ERROR', payload: 'Failed to load session' });
    } finally {
      dispatch({ type: 'SET_LOADING', payload: false });
    }
  };

  const createSession = async (name: string, sessionType: string, model: string) => {
    try {
      let session;
      if (sessionType === 'agent') {
        // Use agent-specific endpoint
        session = await memstApi.createAgentSession({ name, model });
      } else {
        session = await memstApi.createSession({ name, session_type: sessionType, model });
      }
      dispatch({ type: 'ADD_SESSION', payload: session as Session });
      dispatch({ type: 'SET_CURRENT_SESSION', payload: session as Session });
      dispatch({ type: 'SET_MESSAGES', payload: [] });
      dispatch({ type: 'SET_MEMORIES', payload: { working: [], short: [], long: [] } });
    } catch (err) {
      dispatch({ type: 'SET_ERROR', payload: 'Failed to create session' });
    }
  };

  const deleteSession = async (id: string) => {
    try {
      await memstApi.deleteSession(id);
      dispatch({ type: 'DELETE_SESSION', payload: id });
      // Reload sessions from server to ensure consistency
      await loadSessions();
    } catch (err) {
      dispatch({ type: 'SET_ERROR', payload: 'Failed to delete session' });
    }
  };

  const sendMessage = async (content: string) => {
    if (!state.currentSession) return;

    try {
      // Call chat endpoint (which handles adding both user and assistant messages)
      const assistantMessage = await memstApi.chat(state.currentSession.id, content, state.currentSession.model);
      dispatch({ type: 'ADD_MESSAGE', payload: assistantMessage as Message });

      // Reload all messages to get the full conversation
      const messages = await memstApi.getMessages(state.currentSession.id);
      dispatch({ type: 'SET_MESSAGES', payload: messages as Message[] });

      // Reload memories after chat (working memory is auto-saved)
      const memories = await memstApi.getAllMemories(state.currentSession.id);
      dispatch({ type: 'SET_MEMORIES', payload: memories as { working: Memory[]; short: Memory[]; long: Memory[] } });
    } catch (err) {
      dispatch({ type: 'SET_ERROR', payload: 'Failed to send message' });
    }
  };

  const loadMemories = async (sessionId: string) => {
    try {
      const memories = await memstApi.getAllMemories(sessionId);
      dispatch({ type: 'SET_MEMORIES', payload: memories as { working: Memory[]; short: Memory[]; long: Memory[] } });
    } catch (err) {
      dispatch({ type: 'SET_ERROR', payload: 'Failed to load memories' });
    }
  };

  const refreshMemories = async (sessionId: string) => {
    try {
      const memories = await memstApi.getAllMemories(sessionId);
      dispatch({ type: 'SET_MEMORIES', payload: memories as { working: Memory[]; short: Memory[]; long: Memory[] } });
    } catch (err) {
      dispatch({ type: 'SET_ERROR', payload: 'Failed to refresh memories' });
    }
  };

  const parseKnowledgeGraph = async (sessionId: string) => {
    try {
      await memstApi.parseKnowledgeGraph(sessionId);
      // Reload knowledge graph
      const graph = await memstApi.getKnowledgeGraph(sessionId);
      dispatch({ type: 'SET_KNOWLEDGE_GRAPH', payload: graph as KnowledgeGraph });
    } catch (err) {
      dispatch({ type: 'SET_ERROR', payload: 'Failed to parse knowledge graph' });
    }
  };

  const search = async (query: string) => {
    try {
      const results = await memstApi.search(query);
      console.log('Search results:', results);
      // Handle search results
    } catch (err) {
      dispatch({ type: 'SET_ERROR', payload: 'Search failed' });
    }
  };

  return (
    <AppContext.Provider
      value={{
        state,
        dispatch,
        loadSessions,
        loadSession,
        createSession,
        deleteSession,
        sendMessage,
        loadMemories,
        refreshMemories,
        parseKnowledgeGraph,
        search,
      }}
    >
      {children}
    </AppContext.Provider>
  );
}

export function useApp() {
  const context = useContext(AppContext);
  if (!context) {
    throw new Error('useApp must be used within AppProvider');
  }
  return context;
}
