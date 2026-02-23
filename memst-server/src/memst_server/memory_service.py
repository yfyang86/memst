"""MemoryReloadService - Shared service for memory reload operations.

This module provides a unified interface for reloading memories from messages
for both chat sessions and agent sessions.
"""

import re
from typing import Optional, Dict, Any, List
from datetime import datetime


class MemoryReloadService:
    """Shared service for memory reload operations across chat and agent sessions.

    This class encapsulates the logic for extracting memory-worthy content
    from chat messages and repopulating working memory. It can be used
    by both chat session handlers and agent session handlers.
    """

    # Pattern for detecting memory-worthy content
    IMPORTANT_PATTERNS = [
        r"(?:remember|note|important|don't forget|make sure to)",
        r"(?:my name is|i am|i'm)\s+\w+",
        r"(?:i prefer|i like|i hate|i love)",
        r"(?:user's|my)\s+(?:name|preference|location|timezone)",
        r"(?:set|create|add)\s+(?:a\s+)?(?:reminder|task|event)",
        r"(?:todo|to-do|task).*?:",
        r"(?:@\w+|mentions?)\s*:",
    ]

    # Content patterns to skip (noise)
    SKIP_PATTERNS = [
        r"^[\s\n]*$",
        r"^(?:hello|hi|hey|okay|ok|yes|no|yep|nope)[\s\.\!]*$",
        r"^[\s]*(?:thanks?|thank you|cheers)[\s\!]*$",
        r"^(?:what|how|when|where|why|who|which)\s+\w+.*\?$",
        r"^(?:can you|could you|would you|please)\s+\w+",
    ]

    def __init__(self):
        """Initialize the memory reload service."""
        self._compile_patterns()

    def _compile_patterns(self):
        """Compile regex patterns for efficiency."""
        self._important_regex = [
            re.compile(p, re.IGNORECASE) for p in self.IMPORTANT_PATTERNS
        ]
        self._skip_regex = [
            re.compile(p, re.IGNORECASE) for p in self.SKIP_PATTERNS
        ]

    def is_important_content(self, content: str) -> bool:
        """Check if content contains memory-worthy information.

        Args:
            content: The message content to check.

        Returns:
            True if the content appears to contain important information.
        """
        # Check if it matches important patterns
        for pattern in self._important_regex:
            if pattern.search(content):
                return True

        # Check if it should be skipped (short/insignificant)
        for pattern in self._skip_regex:
            if pattern.match(content):
                return False

        # Content length heuristic: very short or very long may be important
        stripped = content.strip()
        if len(stripped) < 5:
            return False
        if len(stripped) > 100:
            return True

        return False

    def extract_memory_content(self, messages: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        """Extract memory-worthy content from messages.

        Args:
            messages: List of message dictionaries with 'role' and 'content' keys.

        Returns:
            List of memory-worthy content items with metadata.
        """
        memories = []
        seen_contents = set()

        for msg in messages:
            role = msg.get("role", "")
            content = msg.get("content", "")

            if not content:
                continue

            # Skip system messages (they're already in context)
            if role.lower() == "system":
                continue

            # Normalize content for duplicate detection
            normalized = content.lower().strip()[:100]
            if normalized in seen_contents:
                continue
            seen_contents.add(normalized)

            # Check if this is important content
            if self.is_important_content(content):
                memories.append({
                    "content": content,
                    "role": role,
                    "importance": self._calculate_importance(content),
                    "source": "message",
                    "tags": self._extract_tags(content),
                })

        return memories

    def _calculate_importance(self, content: str) -> float:
        """Calculate importance score for content.

        Args:
            content: The message content.

        Returns:
            Importance score between 0.0 and 1.0.
        """
        score = 0.5  # Base score

        # Length-based scoring
        length = len(content.strip())
        if length > 200:
            score += 0.2
        elif length > 500:
            score += 0.3
        elif length < 50:
            score -= 0.1

        # Pattern-based scoring
        important_patterns = [
            r"remember",
            r"important",
            r"don't forget",
            r"preference",
            r"name is",
            r"always",
            r"never",
            r"task",
            r"todo",
            r"deadline",
        ]
        for pattern in important_patterns:
            if re.search(pattern, content, re.IGNORECASE):
                score += 0.1

        # Cap score
        return min(1.0, max(0.0, score))

    def _extract_tags(self, content: str) -> List[str]:
        """Extract tags from content.

        Args:
            content: The message content.

        Returns:
            List of tags extracted from content.
        """
        tags = []

        # Look for explicit tag patterns
        tag_pattern = re.findall(r"#(\w+)", content)
        tags.extend([f"tag:{t.lower()}" for t in tag_pattern])

        # Look for topic indicators
        topic_patterns = [
            (r"(?:about|topic|subject)\s*:?\s*(\w+)", "topic"),
            (r"(?:meeting|call|event)\s*:?\s*([^\n]+)", "event"),
            (r"(?:task|todo)\s*:?\s*([^\n]+)", "task"),
        ]

        for pattern, tag_type in topic_patterns:
            matches = re.findall(pattern, content, re.IGNORECASE)
            for match in matches[:2]:  # Limit tags per message
                if match and len(match) > 2:
                    tags.append(f"{tag_type}:{match.lower()[:20]}")

        return list(set(tags))[:5]  # Limit to 5 tags

    def format_memory_for_storage(
        self,
        content: str,
        role: str,
        importance: float = 0.5,
        tags: Optional[List[str]] = None,
    ) -> Dict[str, Any]:
        """Format extracted content for storage in memory tier.

        Args:
            content: The original message content.
            role: The message role (user/assistant).
            importance: Importance score (0.0-1.0).
            tags: Optional list of tags.

        Returns:
            Dictionary formatted for memory storage.
        """
        return {
            "content": content,
            "source": f"message_{role}",
            "importance": importance,
            "confidence": 0.8,
            "tags": tags or [],
            "created_at": datetime.utcnow().isoformat(),
            "access_count": 0,
        }

    def generate_summary_from_messages(
        self, messages: List[Dict[str, Any]], max_length: int = 500
    ) -> str:
        """Generate a summary from recent messages.

        Args:
            messages: List of message dictionaries.
            max_length: Maximum length of summary.

        Returns:
            Generated summary string.
        """
        # Extract key points from user messages
        key_points = []
        for msg in messages[-20:]:  # Last 20 messages
            if msg.get("role", "").lower() == "user":
                content = msg.get("content", "").strip()
                if content and self.is_important_content(content):
                    # Truncate long messages
                    if len(content) > 100:
                        content = content[:100] + "..."
                    key_points.append(content)

        if not key_points:
            return ""

        # Combine and truncate
        summary = " ".join(key_points)
        if len(summary) > max_length:
            summary = summary[:max_length] + "..."

        return summary
