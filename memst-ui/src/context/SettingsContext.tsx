import { createContext, useContext, useState, useEffect, ReactNode } from 'react';
import { setApiBaseUrl, memstApi } from '../api/client';

export interface LLMConfig {
  type: string;
  api_url: string;
  model: string;
  timeout: number;
  max_tokens: number;
  temperature: number;
  api_key: string;
}

export interface EmbeddingConfig {
  type: string;
  api_url: string;
  model: string;
  timeout: number;
  expected_dimension: number;
}

export interface ServerConfig {
  host: string;
  port: number;
  store_path: string;
}

export interface KGExtractionConfig {
  enabled: boolean;
  db_path: string | null;
  default_ontology: string | null;
}

interface SettingsState {
  llm: LLMConfig;
  embedding: EmbeddingConfig;
  server: ServerConfig;
  kg_extraction: KGExtractionConfig;
}

interface SettingsContextType {
  settings: SettingsState;
  updateLLM: (config: Partial<LLMConfig>) => void;
  updateEmbedding: (config: Partial<EmbeddingConfig>) => void;
  updateServer: (config: Partial<ServerConfig>) => void;
  updateKGExtraction: (config: Partial<KGExtractionConfig>) => void;
  resetSettings: () => Promise<void>;
  saveSettings: () => Promise<void>;
  isLoading: boolean;
  error: string | null;
}

// Fallback defaults if API is not available
const defaultLLM: LLMConfig = {
  type: 'openai',
  api_url: 'http://localhost:8080/v1',
  model: 'gpt-4',
  timeout: 60,
  max_tokens: 8192,
  temperature: 0.7,
  api_key: '',
};

const defaultEmbedding: EmbeddingConfig = {
  type: 'openai',
  api_url: 'http://localhost:8081/v1/embeddings',
  model: 'text-embedding-bge_m3',
  timeout: 30,
  expected_dimension: 1024,
};

const defaultServer: ServerConfig = {
  host: '127.0.0.1',
  port: 8192,
  store_path: './memst-store',
};

const defaultKGExtraction: KGExtractionConfig = {
  enabled: true,
  db_path: null,
  default_ontology: null,
};

const defaultSettings: SettingsState = {
  llm: defaultLLM,
  embedding: defaultEmbedding,
  server: defaultServer,
  kg_extraction: defaultKGExtraction,
};

const SettingsContext = createContext<SettingsContextType | null>(null);

const STORAGE_KEY = 'memst-settings';

function loadSettingsFromStorage(): SettingsState {
  if (typeof window === 'undefined') return defaultSettings;

  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored) {
    try {
      return JSON.parse(stored);
    } catch {
      return defaultSettings;
    }
  }
  return defaultSettings;
}

export function SettingsProvider({ children }: { children: ReactNode }) {
  const [settings, setSettings] = useState<SettingsState>(defaultSettings);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Load settings from API on mount
  useEffect(() => {
    async function loadSettings() {
      try {
        const apiSettings = await memstApi.getSettings();

        // Update API base URL from server settings
        setApiBaseUrl(apiSettings.server.host, apiSettings.server.port);

        const newSettings: SettingsState = {
          llm: {
            type: apiSettings.llm.type,
            api_url: apiSettings.llm.api_url,
            model: apiSettings.llm.model,
            timeout: apiSettings.llm.timeout,
            max_tokens: apiSettings.llm.max_tokens,
            temperature: apiSettings.llm.temperature,
            api_key: apiSettings.llm.api_key,
          },
          embedding: {
            type: apiSettings.embedding.type,
            api_url: apiSettings.embedding.api_url,
            model: apiSettings.embedding.model,
            timeout: apiSettings.embedding.timeout,
            expected_dimension: apiSettings.embedding.expected_dimension,
          },
          server: {
            host: apiSettings.server.host,
            port: apiSettings.server.port,
            store_path: apiSettings.server.store_path,
          },
          kg_extraction: apiSettings.kg_extraction || defaultKGExtraction,
        };

        setSettings(newSettings);
        localStorage.setItem(STORAGE_KEY, JSON.stringify(newSettings));
      } catch (err) {
        console.warn('Failed to load settings from API, using localStorage:', err);
        const stored = loadSettingsFromStorage();
        setSettings(stored);
        // Update API base URL from stored settings
        setApiBaseUrl(stored.server.host, stored.server.port);
      } finally {
        setIsLoading(false);
      }
    }

    loadSettings();
  }, []);

  // Save to localStorage on change
  useEffect(() => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
  }, [settings]);

  const updateLLM = (config: Partial<LLMConfig>) => {
    setSettings((prev) => ({
      ...prev,
      llm: { ...prev.llm, ...config },
    }));
  };

  const updateEmbedding = (config: Partial<EmbeddingConfig>) => {
    setSettings((prev) => ({
      ...prev,
      embedding: { ...prev.embedding, ...config },
    }));
  };

  const updateServer = (config: Partial<ServerConfig>) => {
    setSettings((prev) => {
      const newServer = { ...prev.server, ...config };
      // Update API base URL if host or port changes
      setApiBaseUrl(newServer.host, newServer.port);
      return { ...prev, server: newServer };
    });
  };

  const updateKGExtraction = (config: Partial<KGExtractionConfig>) => {
    setSettings((prev) => ({
      ...prev,
      kg_extraction: { ...prev.kg_extraction, ...config },
    }));
  };

  const resetSettings = async () => {
    try {
      const result = await memstApi.resetSettings() as {
        llm: LLMConfig;
        embedding: EmbeddingConfig;
        server: ServerConfig;
      };
      const newSettings: SettingsState = {
        llm: result.llm,
        embedding: result.embedding,
        server: result.server,
      };
      setSettings(newSettings);
      setApiBaseUrl(newSettings.server.host, newSettings.server.port);
    } catch (err) {
      console.warn('Failed to reset settings via API, using defaults:', err);
      setSettings(defaultSettings);
      setApiBaseUrl(defaultSettings.server.host, defaultSettings.server.port);
    }
  };

  const saveSettings = async () => {
    setError(null);
    try {
      await memstApi.updateSettings({
        llm: settings.llm,
        embedding: settings.embedding,
        server: settings.server,
        kg_extraction: settings.kg_extraction,
      });
    } catch (err) {
      setError('Failed to save settings to server');
      console.error('Failed to save settings:', err);
    }
  };

  return (
    <SettingsContext.Provider
      value={{
        settings,
        updateLLM,
        updateEmbedding,
        updateServer,
        updateKGExtraction,
        resetSettings,
        saveSettings,
        isLoading,
        error,
      }}
    >
      {children}
    </SettingsContext.Provider>
  );
}

export function useSettings() {
  const context = useContext(SettingsContext);
  if (!context) {
    throw new Error('useSettings must be used within SettingsProvider');
  }
  return context;
}
