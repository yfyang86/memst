// API client for MemSt backend

import { getApiBaseUrl as getConfigApiBaseUrl } from "../config";

let API_BASE = getConfigApiBaseUrl();
let currentAbortController: AbortController | null = null;

// Update API base URL dynamically
export function setApiBaseUrl(host: string, port: number) {
  API_BASE = `http://${host}:${port}/api/v1`;
}

export function getApiBaseUrl(): string {
  return API_BASE;
}

// Cancel any ongoing request
export function cancelCurrentRequest() {
  if (currentAbortController) {
    currentAbortController.abort();
    currentAbortController = null;
  }
}

async function fetchApi<T>(
  endpoint: string,
  options?: RequestInit,
): Promise<T> {
  const response = await fetch(`${API_BASE}${endpoint}`, {
    ...options,
    headers: {
      "Content-Type": "application/json",
      ...options?.headers,
    },
  });

  if (!response.ok) {
    throw new Error(`API error: ${response.status}`);
  }

  return response.json();
}

export const memstApi = {
  // Auth
  login: (name: string, avatar?: string) =>
    fetchApi<{ id: string; name: string; avatar?: string }>("/auth/login", {
      method: "POST",
      body: JSON.stringify({ name, avatar }),
    }),

  logout: () => fetchApi("/auth/logout", { method: "POST" }),

  getCurrentUser: () =>
    fetchApi<{ id: string; name: string; avatar?: string }>("/users/me"),

  // Settings
  getSettings: () =>
    fetchApi<{
      llm: {
        type: string;
        api_url: string;
        model: string;
        timeout: number;
        max_tokens: number;
        temperature: number;
        api_key: string;
      };
      embedding: {
        type: string;
        api_url: string;
        model: string;
        timeout: number;
        expected_dimension: number;
      };
      server: { host: string; port: number; store_path: string };
    }>("/settings"),

  updateSettings: (settings: Record<string, unknown>) =>
    fetchApi("/settings", {
      method: "PUT",
      body: JSON.stringify(settings),
    }),

  resetSettings: () => fetchApi("/settings/reset", { method: "POST" }),

  // Sessions
  listSessions: (session_type?: string) =>
    session_type
      ? fetchApi<{ id: string; name: string; session_type: string }[]>(
          `/sessions?session_type=${session_type}`,
        )
      : fetchApi<{ id: string; name: string; session_type: string }[]>(
          "/sessions",
        ),

  getSession: (id: string) => fetchApi(`/sessions/${id}`),

  createSession: (data: {
    name: string;
    session_type: string;
    model: string;
    tags?: string[];
  }) =>
    fetchApi("/sessions", {
      method: "POST",
      body: JSON.stringify(data),
    }),

  deleteSession: (id: string) =>
    fetchApi(`/sessions/${id}`, { method: "DELETE" }),

  // Messages
  getMessages: (sessionId: string, limit = 100, offset = 0) =>
    fetchApi(`/sessions/${sessionId}/messages?limit=${limit}&offset=${offset}`),

  addMessage: (sessionId: string, role: string, content: string) =>
    fetchApi(`/sessions/${sessionId}/messages`, {
      method: "POST",
      body: JSON.stringify({ role, content }),
    }),

  // Chat (LLM)
  chat: (sessionId: string, message: string, model?: string) =>
    fetchApi<{
      id: string;
      session_id: string;
      role: string;
      content: string;
      timestamp: string;
      metadata: { model: string; usage?: Record<string, unknown> };
    }>(`/sessions/${sessionId}/chat`, {
      method: "POST",
      body: JSON.stringify({ message, model }),
    }),

  // Chat streaming
  chatStream: async function* (
    sessionId: string,
    message: string,
    model?: string,
  ): AsyncGenerator<string> {
    // Cancel any previous request
    cancelCurrentRequest();
    currentAbortController = new AbortController();

    const response = await fetch(
      `${getApiBaseUrl()}/sessions/${sessionId}/chat/stream`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ message, model }),
        signal: currentAbortController.signal,
      },
    );

    if (!response.ok) {
      throw new Error(`API error: ${response.status}`);
    }

    const reader = response.body?.getReader();
    if (!reader) return;

    currentAbortController = null; // Clear on successful connection

    const decoder = new TextDecoder();
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      const chunk = decoder.decode(value);
      const lines = chunk.split("\n");
      for (const line of lines) {
        if (line.startsWith("data: ")) {
          const data = line.slice(6);
          if (data === "[DONE]") break;
          try {
            const parsed = JSON.parse(data);
            if (parsed.content) {
              yield parsed.content;
            }
          } catch (e) {
            // Ignore parse errors
          }
        }
      }
    }
  },

  // Memory
  getAllMemories: (sessionId: string) =>
    fetchApi<{
      working: Array<{
        id: string;
        content: string;
        detail?: string;
        tier: string;
        timestamp?: string;
        round?: number;
      }>;
      short: Array<{
        id: string;
        content: string;
        tier: string;
        file_name?: string;
        timestamp?: string;
      }>;
      long: Array<{
        id: string;
        content: string;
        tier: string;
        summary?: string;
        timestamp?: string;
      }>;
    }>(`/sessions/${sessionId}/memory`),

  getMemoriesByTier: (sessionId: string, tier: string) =>
    fetchApi(`/sessions/${sessionId}/memory/${tier}`),

  addMemory: (
    sessionId: string,
    tier: string,
    content: string,
    tags?: string[],
    importance?: number,
  ) =>
    fetchApi(`/sessions/${sessionId}/memory`, {
      method: "POST",
      body: JSON.stringify({ tier, content, tags, importance }),
    }),

  retrieveMemories: (
    sessionId: string,
    query: string,
    limit?: number,
    tier?: string,
  ) =>
    fetchApi(`/sessions/${sessionId}/memory/retrieve`, {
      method: "POST",
      body: JSON.stringify({ query, limit, tier }),
    }),

  summarizeSession: (sessionId: string) =>
    fetchApi(`/sessions/${sessionId}/memory/summarize`, { method: "POST" }),

  reloadMemory: (sessionId: string) =>
    fetchApi<{
      success: boolean;
      memories_added: number;
      memories_skipped: number;
      message: string;
    }>(`/sessions/${sessionId}/memory/reload`, { method: "POST" }),

  // Search
  search: (
    query: string,
    searchType = "hybrid",
    sessionId?: string,
    limit = 20,
  ) =>
    fetchApi("/search", {
      method: "POST",
      body: JSON.stringify({
        query,
        search_type: searchType,
        session_id: sessionId,
        limit,
      }),
    }),

  hybridSearch: (query: string, sessionId?: string, limit = 20) =>
    memstApi.search(query, "hybrid", sessionId, limit),

  semanticSearch: (query: string, sessionId?: string, limit = 20) =>
    memstApi.search(query, "semantic", sessionId, limit),

  // Files
  listFiles: (sessionId: string) => fetchApi(`/sessions/${sessionId}/files`),

  uploadFile: (sessionId: string, file: File) => {
    const formData = new FormData();
    formData.append("file", file);
    return fetchApi(`/sessions/${sessionId}/files`, {
      method: "POST",
      body: formData,
      headers: {}, // Let browser set Content-Type for multipart
    });
  },

  // Trace
  getTrace: (sessionId: string, opType?: string) => {
    const url = opType
      ? `/sessions/${sessionId}/trace/${opType}`
      : `/sessions/${sessionId}/trace`;
    return fetchApi(url);
  },

  // Knowledge Graph
  getKnowledgeGraph: (sessionId: string) =>
    fetchApi(`/sessions/${sessionId}/knowledge-graph`),

  parseKnowledgeGraph: (sessionId: string) =>
    fetchApi(`/sessions/${sessionId}/knowledge-graph/parse`, {
      method: "POST",
    }),

  // Stats
  getGlobalStats: () => fetchApi("/stats"),

  getSessionStats: (sessionId: string) =>
    fetchApi(`/sessions/${sessionId}/stats`),

  // Health
  healthCheck: () => fetchApi("/health"),

  // Agent (nanobot)
  getAgentStatus: () =>
    fetchApi<{ available: boolean; session_count: number }>("/agent/status"),

  createAgentSession: (data: {
    name: string;
    model?: string;
    tags?: string[];
  }) =>
    fetchApi("/agent/sessions", {
      method: "POST",
      body: JSON.stringify({ ...data, session_type: "agent" }),
    }),

  agentChat: (sessionId: string, message: string, model?: string) =>
    fetchApi<{
      id: string;
      session_id: string;
      role: string;
      content: string;
      timestamp: string;
    }>(`/agent/chat/${sessionId}`, {
      method: "POST",
      body: JSON.stringify({ message, model }),
    }),

  agentChatStream: async function* (
    sessionId: string,
    message: string,
    model?: string,
  ): AsyncGenerator<string> {
    // Cancel any previous request
    cancelCurrentRequest();
    currentAbortController = new AbortController();

    const response = await fetch(
      `${getApiBaseUrl()}/agent/chat/${sessionId}/stream`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ message, model }),
        signal: currentAbortController.signal,
      },
    );

    if (!response.ok) {
      throw new Error(`API error: ${response.status}`);
    }

    const reader = response.body?.getReader();
    if (!reader) return;

    currentAbortController = null; // Clear on successful connection

    const decoder = new TextDecoder();
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      const chunk = decoder.decode(value);
      const lines = chunk.split("\n");
      for (const line of lines) {
        if (line.startsWith("data: ")) {
          const data = line.slice(6);
          if (data === "[DONE]") break;
          try {
            const parsed = JSON.parse(data);
            if (parsed.content) {
              yield parsed.content;
            }
          } catch (e) {
            // Ignore parse errors
          }
        }
      }
    }
  },

  getAgentHistory: (sessionId: string) =>
    fetchApi(`/agent/sessions/${sessionId}/history`),

  deleteAgentSession: (sessionId: string) =>
    fetchApi(`/agent/sessions/${sessionId}`, { method: "DELETE" }),
};
