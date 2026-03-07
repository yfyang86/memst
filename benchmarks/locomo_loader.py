"""
LOCOMO dataset loader for MemSt benchmark.
"""
import json
import re
from pathlib import Path
from typing import List, Dict, Any, Tuple
from dataclasses import dataclass


@dataclass
class Question:
    """Single question from LOCOMO dataset."""
    question: str
    answer: str
    category: int
    evidence: List[str]
    conversation_id: str
    
    @property
    def category_name(self) -> str:
        names = {
            1: "single-hop",
            2: "multi-hop", 
            3: "temporal",
            4: "open-domain",
            5: "adversarial"
        }
        return names.get(self.category, "unknown")


@dataclass
class Conversation:
    """A conversation with multiple sessions."""
    sample_id: str
    speaker_a: str
    speaker_b: str
    sessions: List[Dict[str, Any]]  # List of session turns
    session_dates: List[str]
    questions: List[Question]
    
    def get_all_turns(self) -> List[Dict]:
        """Flatten all sessions into a single list of turns."""
        all_turns = []
        for session in self.sessions:
            all_turns.extend(session)
        return all_turns
    
    def get_full_text(self) -> str:
        """Get full conversation text for full-context baseline."""
        turns = self.get_all_turns()
        return "\n".join([f"{t['speaker']}: {t['text']}" for t in turns])


class LoCoMoLoader:
    """Load and process LOCOMO dataset."""
    
    def __init__(self, data_path: str = "../third/locomo/data/locomo10.json"):
        self.data_path = Path(data_path)
        self.conversations = []
        self._load()
    
    def _load(self):
        """Load LOCOMO data from JSON."""
        with open(self.data_path, 'r') as f:
            data = json.load(f)
        
        for conv_data in data:
            conv = self._parse_conversation(conv_data)
            self.conversations.append(conv)
    
    def _parse_conversation(self, data: Dict) -> Conversation:
        """Parse a single conversation from LOCOMO format."""
        conversation_data = data['conversation']
        
        # Extract speakers
        speaker_a = conversation_data.get('speaker_a', 'Speaker A')
        speaker_b = conversation_data.get('speaker_b', 'Speaker B')
        
        # Extract sessions
        sessions = []
        session_dates = []
        
        session_num = 1
        while f"session_{session_num}" in conversation_data:
            session_key = f"session_{session_num}"
            date_key = f"session_{session_num}_date_time"
            
            session_turns = conversation_data[session_key]
            session_date = conversation_data.get(date_key, "")
            
            sessions.append(session_turns)
            session_dates.append(session_date)
            session_num += 1
        
        # Parse questions
        questions = []
        for qa in data.get('qa', []):
            # Handle adversarial questions which have 'adversarial_answer' instead of 'answer'
            answer = qa.get('answer', qa.get('adversarial_answer', ''))
            q = Question(
                question=qa['question'],
                answer=str(answer),
                category=qa['category'],
                evidence=qa.get('evidence', []),
                conversation_id=data['sample_id']
            )
            questions.append(q)
        
        return Conversation(
            sample_id=data['sample_id'],
            speaker_a=speaker_a,
            speaker_b=speaker_b,
            sessions=sessions,
            session_dates=session_dates,
            questions=questions
        )
    
    def get_conversations(self) -> List[Conversation]:
        """Get all conversations."""
        return self.conversations
    
    def get_questions_by_category(self, category: int) -> List[Question]:
        """Get all questions of a specific category."""
        questions = []
        for conv in self.conversations:
            for q in conv.questions:
                if q.category == category:
                    questions.append(q)
        return questions
    
    def get_statistics(self) -> Dict[str, Any]:
        """Get dataset statistics."""
        stats = {
            'num_conversations': len(self.conversations),
            'total_questions': sum(len(c.questions) for c in self.conversations),
            'category_distribution': {},
            'avg_sessions_per_conv': sum(len(c.sessions) for c in self.conversations) / len(self.conversations),
            'avg_turns_per_conv': sum(len(c.get_all_turns()) for c in self.conversations) / len(self.conversations),
        }
        
        # Category distribution
        for conv in self.conversations:
            for q in conv.questions:
                cat_name = q.category_name
                stats['category_distribution'][cat_name] = stats['category_distribution'].get(cat_name, 0) + 1
        
        return stats


def load_locomo(data_path: str = "../third/locomo/data/locomo10.json") -> LoCoMoLoader:
    """Convenience function to load LOCOMO dataset."""
    return LoCoMoLoader(data_path)


if __name__ == "__main__":
    # Test loader
    loader = load_locomo()
    stats = loader.get_statistics()
    print("LOCOMO Dataset Statistics:")
    print(json.dumps(stats, indent=2))
    
    # Sample question
    conv = loader.conversations[0]
    print(f"\nSample conversation: {conv.sample_id}")
    print(f"Speakers: {conv.speaker_a} and {conv.speaker_b}")
    print(f"Sessions: {len(conv.sessions)}")
    print(f"Questions: {len(conv.questions)}")
    
    if conv.questions:
        q = conv.questions[0]
        print(f"\nSample question: {q.question}")
        print(f"Answer: {q.answer}")
        print(f"Category: {q.category_name}")
