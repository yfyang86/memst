"""
MemSt strategy for LOCOMO benchmark.
Uses MemSt REST API for memory operations.
"""
import requests
import json
import time
from typing import Dict, Any, List, Optional
from strategies.base import BenchmarkStrategy


class MemStStrategy(BenchmarkStrategy):
    """
    MemSt-based retrieval strategy.
    Requires memst-server to be running.
    """
    
    def __init__(
        self,
        api_url: str = "http://127.0.0.1:8193/api/v1",
        use_kg: bool = False,
        use_summaries: bool = False,
        search_mode: str = "hybrid",
        token_budget: int = 2048
    ):
        name = "MemSt"
        if use_kg:
            name += "+KG"
        if use_summaries:
            name += "+Sum"
        
        super().__init__(name)
        self.api_url = api_url
        self.use_kg = use_kg
        self.use_summaries = use_summaries
        self.search_mode = search_mode
        self.token_budget = token_budget
        
        self.session_id = None
        self.conversation_id = None
        self.headers = {"Content-Type": "application/json"}
    
    def _api_call(self, method: str, endpoint: str, data: Dict = None) -> Dict:
        """Make API call to MemSt server."""
        url = f"{self.api_url}{endpoint}"
        try:
            if method == "GET":
                response = requests.get(url, headers=self.headers, timeout=30)
            elif method == "POST":
                response = requests.post(url, headers=self.headers, json=data, timeout=30)
            else:
                raise ValueError(f"Unsupported method: {method}")
            
            response.raise_for_status()
            return response.json()
        except requests.exceptions.RequestException as e:
            print(f"API error: {e}")
            return {"error": str(e)}
    
    def setup(self, conversation) -> None:
        """
        Setup MemSt with conversation data.
        Creates a session and populates it with conversation messages.
        """
        start_time = time.time()
        
        # 1. Create session
        session_data = self._api_call("POST", "/sessions", {
            "name": f"LOCOMO-{conversation.sample_id}",
            "model": "gpt-4o-mini",
            "metadata": {
                "speaker_a": conversation.speaker_a,
                "speaker_b": conversation.speaker_b,
                "source": "locomo_benchmark"
            }
        })
        
        if "error" in session_data:
            print(f"Failed to create session: {session_data['error']}")
            return
        
        self.session_id = session_data.get("id")
        self.conversation_id = conversation.sample_id
        
        # 2. Add messages from all sessions
        turns = conversation.get_all_turns()
        for turn in turns:
            role = "user" if turn['speaker'] == conversation.speaker_a else "assistant"
            self._api_call("POST", f"/sessions/{self.session_id}/messages", {
                "role": role,
                "content": turn['text'],
                "metadata": {
                    "speaker": turn['speaker'],
                    "dia_id": turn.get('dia_id', '')
                }
            })
        
        # 3. Optional: Extract KG
        if self.use_kg:
            self._setup_kg(turns)
        
        setup_time = time.time() - start_time
        print(f"Setup complete in {setup_time:.2f}s: {len(turns)} messages")
    
    def _setup_kg(self, turns: List[Dict]) -> None:
        """Setup Knowledge Graph from conversation."""
        # Combine all text
        full_text = "\n".join([t['text'] for t in turns])
        
        # Load default ontology
        self._api_call("POST", "/kg/ontologies/load", {
            "schema_json": json.dumps([{
                "top_category": "General",
                "first_category": "Personal",
                "second_category": "Life Events",
                "chinese_name": "个人-生活事件",
                "english_name": "Personal-Life Events",
                "overview": "Personal life events and preferences"
            }])
        })
        
        # Extract entities
        self._api_call("POST", "/kg/extract", {
            "doc_id": self.conversation_id,
            "text": full_text[:10000],  # Limit text length
            "ontology_id": "general"
        })
    
    def answer(self, question) -> Dict[str, Any]:
        """
        Answer a question using MemSt retrieval.
        """
        if not self.session_id:
            return {
                'answer': "Error: Session not initialized",
                'context': "",
                'tokens': 0,
                'latency': 0.0
            }
        
        start_time = time.time()
        
        # 1. Search for relevant context
        search_result = self._api_call("POST", "/search", {
            "query": question.question,
            "session_id": self.session_id,
            "mode": self.search_mode,
            "limit": 10
        })
        
        # 2. Get messages from search results
        context_parts = []
        total_tokens = 0
        
        if "results" in search_result:
            for result in search_result["results"]:
                content = result.get("content", "")
                context_parts.append(content)
                # Rough token count
                total_tokens += len(content) // 4
        
        # 3. Optional: Add KG context
        kg_context = ""
        if self.use_kg:
            kg_result = self._api_call("POST", "/kg/search", {
                "query": question.question,
                "limit": 5
            })
            if "entities" in kg_result:
                for entity in kg_result["entities"]:
                    kg_context += f"{entity['name']} ({entity['entity_type']})\n"
        
        # Combine context
        full_context = "\n".join(context_parts)
        if kg_context:
            full_context += f"\n[Knowledge Graph]\n{kg_context}"
        
        latency = time.time() - start_time
        
        # Note: Actual answer generation would use an LLM
        # For now, we return the context for the evaluation framework to use
        result = {
            'answer': "",  # To be filled by LLM
            'context': full_context,
            'tokens': total_tokens,
            'latency': latency
        }
        
        self.metrics['total_queries'] += 1
        self.metrics['total_time'] += latency
        self.metrics['tokens_retrieved'] += total_tokens
        
        return result
    
    def cleanup(self):
        """Clean up session."""
        if self.session_id:
            try:
                requests.delete(f"{self.api_url}/sessions/{self.session_id}")
            except:
                pass


class RAGStrategy(BenchmarkStrategy):
    """
    Simple RAG baseline using vector search.
    Does not use MemSt - implemented for comparison.
    """
    
    def __init__(
        self,
        chunk_size: int = 512,
        overlap: int = 128,
        top_k: int = 2,
        embedding_model: str = "text-embedding-3-small"
    ):
        super().__init__(f"RAG-chunk{chunk_size}-k{top_k}")
        self.chunk_size = chunk_size
        self.overlap = overlap
        self.top_k = top_k
        self.embedding_model = embedding_model
        
        self.chunks = []
        self.chunk_embeddings = []
    
    def setup(self, conversation) -> None:
        """Chunk conversation and compute embeddings."""
        import openai
        
        text = conversation.get_full_text()
        
        # Simple chunking by characters (not ideal, but simple)
        # Better: chunk by sentences or paragraphs
        step = self.chunk_size - self.overlap
        for i in range(0, len(text), step):
            chunk = text[i:i + self.chunk_size]
            if len(chunk) > 100:  # Skip very small chunks
                self.chunks.append(chunk)
        
        # Compute embeddings
        # Note: This requires OpenAI API key
        try:
            client = openai.OpenAI()
            for i in range(0, len(self.chunks), 100):  # Batch in groups of 100
                batch = self.chunks[i:i+100]
                response = client.embeddings.create(
                    model=self.embedding_model,
                    input=batch
                )
                for item in response.data:
                    self.chunk_embeddings.append(item.embedding)
        except Exception as e:
            print(f"Embedding error: {e}")
            # Fallback: use zero embeddings
            self.chunk_embeddings = [[0.0] * 1536 for _ in self.chunks]
    
    def answer(self, question) -> Dict[str, Any]:
        """Retrieve top-k chunks."""
        import openai
        import numpy as np
        
        start_time = time.time()
        
        # Compute question embedding
        try:
            client = openai.OpenAI()
            response = client.embeddings.create(
                model=self.embedding_model,
                input=[question.question]
            )
            query_embedding = np.array(response.data[0].embedding)
        except:
            query_embedding = np.zeros(1536)
        
        # Compute similarities
        if self.chunk_embeddings:
            chunk_embeddings = np.array(self.chunk_embeddings)
            similarities = np.dot(chunk_embeddings, query_embedding) / (
                np.linalg.norm(chunk_embeddings, axis=1) * np.linalg.norm(query_embedding) + 1e-8
            )
            top_indices = np.argsort(similarities)[-self.top_k:]
            
            retrieved_chunks = [self.chunks[i] for i in top_indices]
            context = "\n".join(retrieved_chunks)
            tokens = sum(len(c) for c in retrieved_chunks) // 4
        else:
            context = ""
            tokens = 0
        
        latency = time.time() - start_time
        
        result = {
            'answer': "",
            'context': context,
            'tokens': tokens,
            'latency': latency
        }
        
        self.metrics['total_queries'] += 1
        self.metrics['total_time'] += latency
        self.metrics['tokens_retrieved'] += tokens
        
        return result


if __name__ == "__main__":
    # Test strategy
    from locomo_loader import load_locomo
    
    loader = load_locomo()
    conv = loader.conversations[0]
    
    print("Testing MemStStrategy...")
    strategy = MemStStrategy(use_kg=False)
    
    # Check if server is running
    try:
        response = requests.get("http://127.0.0.1:8193/api/v1/health", timeout=2)
        if response.status_code == 200:
            print("MemSt server is running")
            strategy.setup(conv)
            
            if conv.questions:
                q = conv.questions[0]
                result = strategy.answer(q)
                print(f"Question: {q.question}")
                print(f"Context length: {len(result['context'])} chars")
                print(f"Tokens: {result['tokens']}")
                print(f"Latency: {result['latency']:.3f}s")
            
            strategy.cleanup()
        else:
            print("MemSt server not responding correctly")
    except requests.exceptions.ConnectionError:
        print("MemSt server not running. Start it with: ./server.sh --start")
