// Type definitions for MemSt frontend

export interface User {
  id: string;
  name: string;
  avatar?: string;
  role?: string;
  created_at: string;
  settings?: UserSettings;
}

export interface UserSettings {
  theme: 'dark' | 'light';
  default_model: string;
  store_path: string;
}

export interface Session {
  id: string;
  name: string;
  session_type: 'chat' | 'search' | 'task' | 'recommend' | 'agent';
  model: string;
  user_id?: string;
  created_at: string;
  updated_at: string;
  message_count: number;
  tags: string[];
  status: 'active' | 'archived';
}

export interface Message {
  id: string;
  session_id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  timestamp: string;
  attachments?: Attachment[];
  metadata?: Record<string, unknown>;
  agent_trace?: AgentTrace[];
}

export interface Attachment {
  id: string;
  name: string;
  type: string;
  size: number;
}

export interface AgentTrace {
  id: string;
  type: string;
  title: string;
  detail: string;
  timestamp: string;
  status: 'success' | 'pending' | 'error';
}

export interface Memory {
  id: string;
  tier: 'working' | 'short' | 'long';
  content: string;
  source?: string;
  tags?: string[];
  confidence?: number;
  importance?: number;
  access_count?: number;
  created_at?: string;
  timestamp?: string;
  round?: number;
  detail?: string;
  summary?: string;
  file_name?: string;
}

export interface SearchResult {
  id: string;
  session_id: string;
  doc_type: string;
  content: string;
  score: number;
  highlight?: string;
  knowledge_graph?: KnowledgeGraphContext;
}

export interface KnowledgeGraphContext {
  related_concepts: string[];
  connections_found: number;
  entities: Entity[];
}

// KG Extraction v2 Types

export interface Ontology {
  id: string;
  top_category: string;
  first_category: string;
  second_category: string;
  chinese_name: string;
  english_name: string;
  overview?: string;
}

export interface ExtractionJob {
  id: string;
  doc_id: string;
  ontology_id: string;
  status: 'pending' | 'running' | 'completed' | 'failed';
  entity_count: number;
  relationship_count: number;
  tokens_used: number;
}

export interface ExtractedEntity {
  id: string;
  name: string;
  entity_type: string;
  confidence: number;
  doc_id?: string;
  ontology_id?: string;
}

export interface KGExtractionStatus {
  available: boolean;
  ontologies_loaded: number;
  version: string;
}

export interface Entity {
  name: string;
  type: string;
  relevance: number;
}

export interface KnowledgeGraph {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

export interface GraphNode {
  id: string;
  label: string;
  type: 'concept' | 'tool' | 'entity';
  connections: number;
  tier?: 'working' | 'short' | 'long';
}

export interface GraphEdge {
  from: string;
  to: string;
  label: string;
}

export interface File {
  id: string;
  name: string;
  type: 'pdf' | 'doc' | 'img' | 'code';
  size: number;
}

export interface Trace {
  id: string;
  type: string;
  title: string;
  detail: string;
  timestamp: string;
  status: 'success' | 'pending' | 'error';
}

export interface Stats {
  total_sessions: number;
  total_messages: number;
  total_memories: number;
  storage_used_mb: number;
  search_queries_today: number;
  avg_response_time_ms: number;
}

export type CanvasTab = 'memory' | 'trace' | 'files' | 'graph' | 'kg-extraction';

export type SidebarTab = 'sessions' | 'resources' | 'history';
