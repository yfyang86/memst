"""Nanobot client wrapper for agent-based conversations."""

import sys
import re
from pathlib import Path
from typing import Optional, List, Dict, Any
from datetime import datetime
import json

# Add nanobot to path
NANOBOT_PATH = Path(__file__).parent.parent.parent / "third" / "nanobot"
if str(NANOBOT_PATH) not in sys.path:
    sys.path.insert(0, str(NANOBOT_PATH))


NANOBOT_AVAILABLE = False
AgentLoop = None
LiteLLMProvider = None
MessageBus = None


# Global config path override
_NANOBOT_CONFIG_PATH: str | None = None


def set_nanobot_config_path(path: str | None) -> None:
    """Set a custom nanobot config path. Use None to reset to default."""
    global _NANOBOT_CONFIG_PATH
    _NANOBOT_CONFIG_PATH = path
    # Clear cached config when path changes
    global _nanobot_client  # noqa: F824
    if _nanobot_client is not None:
        _nanobot_client._config = None


try:
    from nanobot.agent.loop import AgentLoop
    from nanobot.providers.litellm_provider import LiteLLMProvider
    from nanobot.bus.queue import MessageBus
    from nanobot.agent.skills import SkillsLoader
    from nanobot.config.loader import Config as NanoConfig
    NANOBOT_AVAILABLE = True
except ImportError:
    print("Warning: nanobot not available. Install with: cd third/nanobot && uv pip install -e .")


# Pattern to match function calls in channel format
# <|channel|>commentary to=functions.xxx <|constrain|>json<|message|>...{/message|}
CHANNEL_PATTERN = re.compile(r'<\|channel\|>(.*?)<\|constrain\|>(.*?)<\|message\|>(.*?)(?:</message\|>|$)', re.DOTALL)


def _extract_function_calls_and_results(content: str) -> tuple[List[Dict[str, Any]], str]:
    """Extract function calls, results, and clean content from nanobot channel format.

    Returns:
    - function_calls: List of function call info dictionaries
    - clean_content: Human-readable content with tool execution results included
    """
    function_calls = []

    # Pattern to match function calls in channel format
    # <|channel|>commentary to=functions.xxx <|constrain|>json<|message|>...{/message|}
    channel_pattern = r'<\|channel\|>(.*?)<\|constrain\|>(.*?)<\|message\|>(.*?)(?:</message\|>|$)'

    matches = list(re.finditer(channel_pattern, content, re.DOTALL))

    if not matches:
        return [], content

    # Build clean content with execution results
    result_parts = []
    for match in matches:
        channel_type = (match.group(1) or "").strip()
        _constrain = (match.group(2) or "").strip()  # noqa: F841
        message_body = (match.group(3) or "").strip()

        # Extract function name
        func_match = re.search(r'to=functions[./](\w+)', channel_type)
        if not func_match:
            continue

        func_name = func_match.group(1)

        # Parse parameters
        params = {}
        try:
            params = json.loads(message_body) if message_body else {}
        except (json.JSONDecodeError, TypeError):
            pass

        # Extract key info for description
        description = f"Tool: {func_name}"
        if 'path' in params:
            description += f" (path: {params['path']})"
        if 'command' in params:
            description += f" (command: {params['command'][:50]}...)"

        function_calls.append({
            "function_name": func_name,
            "parameters": params,
            "constrain": _constrain,
        })

        result_parts.append(f"[Tool: {func_name}]")

    # Return both function calls and clean content
    if result_parts:
        clean_content = " | ".join(result_parts) + " | " + content[:200].split('<|channel|>')[0]
        return function_calls, clean_content.strip()

    return function_calls, content


def _strip_channel_format(content: str) -> str:
    """Strip nanobot channel format from content for human-readable storage.

    Converts <|channel|>commentary to=functions.xxx <|constrain|>json<|message|>...{/message|}
    into human-readable descriptions like "Called function xxx with params: ..."
    """
    if not content:
        return content

    # Pattern for channel format: <|channel|>...<|constrain|>...<|message|>...{/message|}
    channel_pattern = r'<\|channel\|>(.*?)<\|constrain\|>(.*?)<\|message\|>(.*?)(?:</message\|>|$)'
    matches = list(re.finditer(channel_pattern, content, re.DOTALL))

    if not matches:
        return content

    # Build clean content by replacing channel format with descriptions
    result = content
    for match in reversed(matches):  # Reverse to not mess up positions
        channel_type = (match.group(1) or "").strip()
        _constrain = (match.group(2) or "").strip()  # noqa: F841
        message_body = (match.group(3) or "").strip()

        # Extract function name from channel type (handles both dot and slash separators)
        func_match = re.search(r'to=functions[./](\w+)', channel_type)
        func_name = func_match.group(1) if func_match else "unknown"

        # Try to extract key info from JSON message body
        description = f"Called function: {func_name}"
        try:
            msg_data = json.loads(message_body)
            # Extract key fields based on function
            if 'path' in msg_data:
                description += f" (path: {msg_data['path']})"
            if 'content' in msg_data:
                # Truncate content preview
                content_preview = msg_data['content'][:200].replace('\n', ' ')
                description += f" - {content_preview}..."
        except (json.JSONDecodeError, TypeError):
            pass

        # Replace the entire channel format with the description
        result = result[:match.start()] + description + result[match.end():]

    return result.strip()


def _extract_meaningful_content(user_msg: str, assistant_msg: str) -> str:
    """Extract meaningful summary for working memory.

    Returns a clean summary of the conversation without JSON wrapping.
    """
    # Strip channel format from assistant message
    clean_assistant = _strip_channel_format(assistant_msg)

    # Create a concise summary
    # For tool calls, summarize what was done
    if '<|channel|>' in assistant_msg:
        # Extract function call summary
        func_match = re.search(r'to=functions\.(\w+)', assistant_msg)
        if func_match:
            func_name = func_match.group(1)
            # Try to get more context from user message
            return f"Agent called {func_name} in response to: {user_msg[:100]}..."

    # For regular responses, use a preview
    if len(clean_assistant) > 300:
        clean_assistant = clean_assistant[:300] + "..."

    return f"User: {user_msg[:100]}...\nAgent: {clean_assistant}"


def _has_channel_format(content: str) -> bool:
    """Check if content contains nanobot channel format."""
    return '<|channel|>' in content


def _parse_channel_function_calls(content: str) -> List[Dict[str, Any]]:
    """Parse function calls from nanobot channel format.

    Returns list of function call info dictionaries with:
    - function_name: name of the function to call
    - parameters: parsed JSON parameters
    """
    function_calls = []

    matches = list(CHANNEL_PATTERN.finditer(content))

    for match in matches:
        channel_type = (match.group(1) or "").strip()
        message_body = (match.group(3) or "").strip()

        # Extract function name
        func_match = re.search(r'to=functions[./](\w+)', channel_type)
        if not func_match:
            continue

        func_name = func_match.group(1)

        # Parse parameters from JSON message body
        params = {}
        if message_body:
            try:
                params = json.loads(message_body)
            except (json.JSONDecodeError, TypeError):
                pass

        function_calls.append({
            "function_name": func_name,
            "parameters": params,
        })

    return function_calls


class NanobotClient:
    """Client wrapper for nanobot agent interactions."""

    def __init__(self, workspace: Path = Path("./workspace")):
        self.workspace = workspace
        self.workspace.mkdir(parents=True, exist_ok=True)
        self._agents: Dict[str, Any] = {}  # Cache agents by model
        self._config: Optional[NanoConfig] = None  # noqa: F821
        self._session_history: Dict[str, List[Dict[str, Any]]] = {}  # Cache message history by session_id
        self._skills_loader: Optional[SkillsLoader] = None  # Skills loader instance

    def _get_skills_loader(self) -> Optional[SkillsLoader]:
        """Get or create the skills loader."""
        if self._skills_loader is None and SkillsLoader is not None:
            from pathlib import Path
            # Get the nanobot skills directory (built-in skills)
            nanobot_path = Path(__file__).parent.parent.parent / "third" / "nanobot"
            builtin_skills_dir = nanobot_path / "nanobot" / "skills"
            self._skills_loader = SkillsLoader(self.workspace, builtin_skills_dir)
        return self._skills_loader

    @property
    def is_available(self) -> bool:
        """Check if nanobot is available."""
        return NANOBOT_AVAILABLE

    def _load_config(self) -> NanoConfig:  # noqa: F821
        """Load nanobot config from default location or custom path."""
        if self._config is None:
            from nanobot.config.loader import load_config
            from pathlib import Path

            if _NANOBOT_CONFIG_PATH:
                config_path = Path(_NANOBOT_CONFIG_PATH)
                if config_path.exists():
                    self._config = load_config(config_path)
                else:
                    print(f"Warning: Custom config path not found: {config_path}")
                    self._config = load_config()
            else:
                self._config = load_config()
        return self._config

    def _get_provider_for_model(self, model: str) -> Any:
        """Get or create a LiteLLMProvider for the given model."""
        nanobot_config = self._load_config()

        # Get API key from nanobot config
        api_key = nanobot_config.get_api_key()
        api_base = nanobot_config.get_api_base()

        # Check for model-specific provider config
        model_lower = model.lower()

        # Override based on model name patterns
        if "anthropic" in model_lower:
            api_key = nanobot_config.providers.anthropic.api_key or api_key
            api_base = None  # Anthropic doesn't use api_base
        elif "openai" in model_lower or "gpt" in model_lower:
            api_key = nanobot_config.providers.openai.api_key or api_key
            api_base = nanobot_config.providers.openai.api_base or api_base
        elif "openrouter" in model_lower or model_lower.startswith("sk-or-"):
            api_key = nanobot_config.providers.openrouter.api_key or api_key
            api_base = nanobot_config.providers.openrouter.api_base or api_base
        elif "vllm" in model_lower or "hosted_vllm" in model_lower:
            api_base = nanobot_config.providers.vllm.api_base or api_base
            api_key = nanobot_config.providers.vllm.api_key or api_key

        return LiteLLMProvider(
            api_base=api_base,
            api_key=api_key,
            default_model=model,
        )

    def _get_agent(self, model: str) -> Any:
        """Get or create an agent loop for the given model."""
        if not NANOBOT_AVAILABLE:
            raise RuntimeError("Nanobot is not available")

        # Check cache first
        if model in self._agents:
            return self._agents[model]

        # Create new agent for this model
        provider = self._get_provider_for_model(model)
        bus = MessageBus()

        agent = AgentLoop(
            bus=bus,
            provider=provider,
            workspace=self.workspace,
            model=model,
        )

        # Cache it
        self._agents[model] = agent
        return agent

    async def create_agent_session(self, name: str, model: str = "gpt-4") -> Dict[str, Any]:
        """Create a new agent session and persist to MemSt."""
        if not NANOBOT_AVAILABLE:
            return None

        session_id = f"agent-{datetime.now().strftime('%Y%m%d%H%M%S')}"

        session_info = {
            "id": session_id,
            "name": name,
            "model": model,
            "session_type": "agent",
            "created_at": datetime.now().isoformat(),
            "updated_at": datetime.now().isoformat(),
            "message_count": 0,
            "status": "active",
        }

        # Persist to MemSt for persistence across restarts
        try:
            from memst_server.memst_client import get_memst_client
            memst_client = get_memst_client()
            if memst_client.is_available:
                memst_client.create_session(name, model)
        except Exception as e:
            print(f"Warning: Failed to persist agent session to MemSt: {e}")

        return session_info

    async def chat(
        self,
        session_id: str,
        message: str,
        model: str = "gpt-4",
    ) -> Dict[str, Any]:
        """
        Send a message to the agent and get response.

        Args:
            session_id: The session ID
            message: The user message
            model: The model to use

        Returns:
            Dict with user_message, assistant_message, and agent_trace
        """
        if not NANOBOT_AVAILABLE:
            raise RuntimeError("Nanobot is not available")

        agent = self._get_agent(model)

        # Create session key for nanobot
        session_key = f"memst:{session_id}"

        # Process the message using process_direct
        raw_response = await agent.process_direct(message, session_key)

        # Check if response contains channel format (function call indicators)
        if _has_channel_format(raw_response):
            # Parse function calls from channel format
            function_calls = _parse_channel_function_calls(raw_response)

            if function_calls:
                # Execute tools manually and collect results
                tool_results = []
                for func_call in function_calls:
                    func_name = func_call["function_name"]
                    params = func_call["parameters"]

                    try:
                        # Execute tool via agent's tool registry
                        result = await agent.tools.execute(func_name, params)
                        tool_results.append({
                            "function": func_name,
                            "parameters": params,
                            "result": result,
                        })
                    except Exception as e:
                        tool_results.append({
                            "function": func_name,
                            "parameters": params,
                            "error": str(e),
                        })

                # Build final response with tool execution results
                # Format: Tool results followed by the LLM's analysis
                final_parts = []

                for tr in tool_results:
                    if "error" in tr:
                        final_parts.append(f"[Tool: {tr['function']}] Error: {tr['error']}")
                    else:
                        final_parts.append(f"[Tool: {tr['function']}] {tr['result']}")

                # Extract the LLM's textual content (strip channel format)
                llm_content = _strip_channel_format(raw_response)
                if llm_content:
                    final_parts.append(llm_content)

                final_response = "\n\n".join(final_parts)
            else:
                final_response = _strip_channel_format(raw_response)
        else:
            final_response = raw_response

        # Create message objects
        user_msg = {
            "id": f"user-{datetime.now().timestamp()}",
            "session_id": session_id,
            "role": "user",
            "content": message,
            "timestamp": datetime.now().isoformat(),
        }

        assistant_msg = {
            "id": f"asst-{datetime.now().timestamp()}",
            "session_id": session_id,
            "role": "assistant",
            "content": final_response,
            "timestamp": datetime.now().isoformat(),
        }

        # Store in our history cache
        if session_id not in self._session_history:
            self._session_history[session_id] = []
        self._session_history[session_id].extend([user_msg, assistant_msg])

        # Also sync to MemSt if available
        from memst_server.memst_client import get_memst_client
        memst_client = get_memst_client()
        if memst_client.is_available:
            try:
                # Ensure session exists in MemSt
                if memst_client.get_session(session_id) is None:
                    memst_client.create_session(f"Agent Session {session_id[:8]}", model)

                # Add user message to MemSt
                memst_client.add_message(session_id, "user", message)

                # Store tool execution results
                if _has_channel_format(raw_response):
                    function_calls = _parse_channel_function_calls(raw_response)
                    for func_call in function_calls:
                        func_content = json.dumps(func_call)
                        memst_client.add_message(session_id, "function", func_content)

                # Add assistant message with execution results
                memst_client.add_message(session_id, "assistant", final_response)

                # Save meaningful summary to working memory
                _save_to_working_memory(memst_client, session_id, message, final_response)
            except Exception as e:
                print(f"Warning: Failed to sync agent messages to MemSt: {e}")

        return {
            "user_message": user_msg,
            "assistant_message": assistant_msg,
            "agent_trace": [],
        }

    async def chat_stream(
        self,
        session_id: str,
        message: str,
        model: str = "gpt-4",
    ):
        """
        Send a message to the agent and yield chunks of the response.

        This is a generator that yields content chunks.
        """
        if not NANOBOT_AVAILABLE:
            raise RuntimeError("Nanobot is not available")

        agent = self._get_agent(model)
        session_key = f"memst:{session_id}"

        # Get the session and its history
        session = agent.sessions.get_or_create(session_key)
        messages = session.get_history()

        # Build initial messages
        initial_messages = agent.context.build_messages(
            history=messages,
            current_message=message,
        )

        # Get the provider (cached for this model)
        provider = self._get_provider_for_model(model)

        # Stream the LLM response
        async for chunk in provider.chat_stream(
            messages=initial_messages,
            tools=agent.tools.get_definitions(),
            model=model,
        ):
            yield chunk

    def get_session_history(self, session_id: str, max_messages: int = 50) -> List[Dict[str, Any]]:
        """Get the message history for a session.

        Returns messages from MemSt (persistent storage) as primary source,
        falling back to cached history.
        """
        # First try MemSt as the persistent source
        from memst_server.memst_client import get_memst_client
        memst_client = get_memst_client()

        if memst_client.is_available:
            try:
                memst_messages = memst_client.get_messages(session_id)
                if memst_messages:
                    return memst_messages[-max_messages:] if max_messages > 0 else memst_messages
            except Exception:
                pass

        # Fallback to cached history (in-memory, lost on restart)
        if session_id in self._session_history:
            history = self._session_history[session_id]
            return history[-max_messages:] if max_messages > 0 else history

        # Try to get from agent sessions
        session_key = f"memst:{session_id}"
        for agent in self._agents.values():
            try:
                if hasattr(agent.sessions, 'get'):
                    try:
                        session = agent.sessions.get(session_key)
                        if session:
                            return session.get_history(max_messages)
                    except Exception:
                        pass
            except Exception:
                continue

        return []

    def delete_session(self, session_id: str) -> bool:
        """Delete an agent session."""
        if not NANOBOT_AVAILABLE:
            return False

        # Clear from our history cache
        if session_id in self._session_history:
            del self._session_history[session_id]

        # Try to delete from any of the cached agents
        session_key = f"memst:{session_id}"
        for agent in self._agents.values():
            try:
                if agent.sessions.delete(session_key):
                    return True
            except Exception:
                continue

        return False

    # Skills methods
    def list_skills(self, filter_unavailable: bool = True) -> List[Dict[str, Any]]:
        """List all available skills.

        Args:
            filter_unavailable: If True, filter out skills with unmet requirements.

        Returns:
            List of skill info dicts with 'name', 'path', 'source', 'description'.
        """
        loader = self._get_skills_loader()
        if loader is None:
            return []

        try:
            skills = loader.list_skills(filter_unavailable=filter_unavailable)
            result = []
            for s in skills:
                meta = loader.get_skill_metadata(s["name"]) or {}
                result.append({
                    "name": s["name"],
                    "path": s["path"],
                    "source": s["source"],
                    "description": meta.get("description", ""),
                    "always": meta.get("always", False),
                })
            return result
        except Exception as e:
            print(f"Error listing skills: {e}")
            return []

    def load_skill(self, name: str) -> Optional[str]:
        """Load a skill by name.

        Args:
            name: Skill name.

        Returns:
            Skill content or None if not found.
        """
        loader = self._get_skills_loader()
        if loader is None:
            return None

        try:
            return loader.load_skill(name)
        except Exception as e:
            print(f"Error loading skill {name}: {e}")
            return None

    def get_skills_summary(self) -> str:
        """Get XML-formatted summary of all available skills.

        Returns:
            XML string describing all skills with their availability.
        """
        loader = self._get_skills_loader()
        if loader is None:
            return ""

        try:
            return loader.build_skills_summary()
        except Exception as e:
            print(f"Error building skills summary: {e}")
            return ""

    def get_always_skills(self) -> List[str]:
        """Get skills marked as 'always' that meet requirements.

        Returns:
            List of skill names.
        """
        loader = self._get_skills_loader()
        if loader is None:
            return []

        try:
            return loader.get_always_skills()
        except Exception as e:
            print(f"Error getting always skills: {e}")
            return []


# Global client instance
_nanobot_client: Optional[NanobotClient] = None


def get_nanobot_client() -> NanobotClient:
    """Get global nanobot client instance."""
    global _nanobot_client
    if _nanobot_client is None:
        from memst_server.config import get_config
        config = get_config()

        # Create workspace directory for agent files
        workspace = Path(config.server.store_path) / "agent-workspace"
        workspace.mkdir(parents=True, exist_ok=True)

        _nanobot_client = NanobotClient(workspace=workspace)
    return _nanobot_client


def get_nanobot_config() -> Dict[str, Any]:
    """Get current nanobot configuration info for frontend API."""
    global _NANOBOT_CONFIG_PATH  # noqa: F824
    from pathlib import Path

    # Get default config path
    default_config_path = Path.home() / ".nanobot" / "config.json"
    custom_path = _NANOBOT_CONFIG_PATH
    active_path = custom_path or str(default_config_path)

    # Load config and get info
    client = get_nanobot_client()
    nanobot_config = client._load_config()

    # Get available tools
    tools = []
    if hasattr(nanobot_config, 'tools') and nanobot_config.tools:
        for tool in nanobot_config.tools:
            tools.append({
                "name": getattr(tool, 'name', str(tool)),
                "enabled": getattr(tool, 'enabled', True),
            })

    # Get providers info
    providers = {}
    if hasattr(nanobot_config, 'providers'):
        for name, provider in nanobot_config.providers.__dict__.items():
            if provider and hasattr(provider, 'api_key') and provider.api_key:
                providers[name] = {
                    "api_key_set": True,
                    "api_base": getattr(provider, 'api_base', None),
                }

    return {
        "config_path": active_path,
        "default_config_path": str(default_config_path),
        "custom_config_path": custom_path,
        "config_exists": Path(active_path).exists(),
        "model": getattr(nanobot_config, 'model', 'gpt-4'),
        "providers": providers,
        "tools": tools,
        "tool_count": len(tools),
    }


def reload_nanobot_config() -> bool:
    """Force reload of nanobot config (clears cache)."""
    global _nanobot_client  # noqa: F824
    if _nanobot_client is not None:
        _nanobot_client._config = None
    return True


def _save_to_working_memory(memst_client, session_id: str, user_msg: str, assistant_msg: str):
    """Save user/assistant exchange to working memory for memory tier display.

    This extracts meaningful summary from the conversation and saves it to the
    working memory tier so it shows up in the Context Panel -> Memory.
    """
    # Extract meaningful content (clean summary without channel format)
    summary = _extract_meaningful_content(user_msg, assistant_msg)

    try:
        # Save to working memory tier with clean summary content
        # Note: MemSt stores memories with tiers (working/short/long)
        if hasattr(memst_client, 'add_memory'):
            memst_client.add_memory(session_id, "working", summary)
    except Exception as e:
        print(f"Warning: Failed to save to working memory: {e}")
