#!/usr/bin/env python3
"""
KG Extraction Demo using MemSt LLM service

This demo shows how to extract Knowledge Graph entities, relationships,
and events from various text scenarios (movies, meetings, medical, technical).

Usage:
    python kg_extraction_demo.py --scenario 01_the_matrix
    python kg_extraction_demo.py --scenario 02_tech_meeting
    python kg_extraction_demo.py --all
"""

import argparse
import json
import os
import sys
from dataclasses import dataclass, asdict
from typing import List, Optional

# Add parent directory to path for importing llm.py
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from llm import completions_one_shot


@dataclass
class ExtractedEntity:
    name: str
    entity_type: str
    attributes: dict
    confidence: float
    temporal_relevance: str


@dataclass
class ExtractedRelationship:
    subject: str
    predicate: str
    obj: str  # 'object' is reserved keyword
    confidence: float
    temporal_type: str


@dataclass
class ExtractedEvent:
    name: str
    event_type: str
    participants: List[str]
    timestamp: Optional[str]
    attributes: dict
    confidence: float


@dataclass
class KgExtractionResult:
    entities: List[ExtractedEntity]
    relationships: List[ExtractedRelationship]
    events: List[ExtractedEvent]
    overall_confidence: float


KG_EXTRACTION_PROMPT = """Extract knowledge graph elements from the following text.

Extract:
1. **Entities**: People, organizations, locations, concepts, technologies, projects, conditions, medications
2. **Relationships**: How entities relate (subject-predicate-object)
3. **Events**: Actions, meetings, decisions, accidents

For each entity, determine temporal relevance:
- "permanent": Core facts, definitions
- "long_term": Characters, stable preferences, ongoing projects  
- "short_term": Current tasks, temporary situations
- "temporary": Transient events, momentary states

Output STRICTLY as JSON:
{
  "entities": [
    {
      "name": "entity name",
      "type": "person|organization|location|concept|technology|project|condition|medication|object",
      "attributes": {},
      "confidence": 0.0-1.0,
      "temporal_relevance": "permanent|long_term|short_term|temporary"
    }
  ],
  "relationships": [
    {
      "subject": "entity name",
      "predicate": "relationship verb",
      "object": "entity name",
      "confidence": 0.0-1.0,
      "temporal_type": "permanent|temporary"
    }
  ],
  "events": [
    {
      "name": "event description",
      "type": "meeting|decision|action|accident|interaction",
      "participants": ["names"],
      "timestamp": "ISO8601 or null",
      "attributes": {},
      "confidence": 0.0-1.0
    }
  ],
  "overall_confidence": 0.0-1.0
}

Text to analyze:
"""


def extract_kg_from_text(text: str) -> KgExtractionResult:
    """Extract KG from text using LLM."""
    prompt = KG_EXTRACTION_PROMPT + text + '"""\n\nRespond with ONLY the JSON object:'
    
    response = completions_one_shot(prompt, temperature=0.1, max_tokens=3000)
    
    # Extract content from response
    content = response.get("choices", [{}])[0].get("text", "")
    
    # Try to parse JSON
    try:
        data = json.loads(content)
    except json.JSONDecodeError:
        # Try to find JSON in the response
        start = content.find('{')
        end = content.rfind('}')
        if start != -1 and end != -1:
            data = json.loads(content[start:end+1])
        else:
            raise ValueError("Could not parse JSON response")
    
    # Parse entities
    entities = [
        ExtractedEntity(
            name=e["name"],
            entity_type=e["type"],
            attributes=e.get("attributes", {}),
            confidence=e.get("confidence", 0.5),
            temporal_relevance=e.get("temporal_relevance", "short_term")
        )
        for e in data.get("entities", [])
    ]
    
    # Parse relationships
    relationships = [
        ExtractedRelationship(
            subject=r["subject"],
            predicate=r["predicate"],
            obj=r["object"],
            confidence=r.get("confidence", 0.5),
            temporal_type=r.get("temporal_type", "temporary")
        )
        for r in data.get("relationships", [])
    ]
    
    # Parse events
    events = [
        ExtractedEvent(
            name=e["name"],
            event_type=e["type"],
            participants=e.get("participants", []),
            timestamp=e.get("timestamp"),
            attributes=e.get("attributes", {}),
            confidence=e.get("confidence", 0.5)
        )
        for e in data.get("events", [])
    ]
    
    return KgExtractionResult(
        entities=entities,
        relationships=relationships,
        events=events,
        overall_confidence=data.get("overall_confidence", 0.5)
    )


def load_scenario(scenario_name: str) -> str:
    """Load scenario text from file."""
    scenario_map = {
        "matrix": "01_the_matrix.md",
        "tech": "02_tech_meeting.md",
        "medical": "03_medical_consultation.md",
        "architecture": "04_software_architecture.md",
    }
    
    filename = scenario_map.get(scenario_name, f"{scenario_name}.md")
    filepath = os.path.join("kg_test_scenarios", filename)
    
    if not os.path.exists(filepath):
        raise FileNotFoundError(f"Scenario file not found: {filepath}")
    
    with open(filepath, 'r') as f:
        return f.read()


def print_results(result: KgExtractionResult, scenario_name: str):
    """Pretty print extraction results."""
    print("\n" + "="*70)
    print(f"KG EXTRACTION RESULTS: {scenario_name.upper()}")
    print("="*70)
    print(f"Overall Confidence: {result.overall_confidence:.2%}\n")
    
    # Entities
    print(f"ENTITIES ({len(result.entities)}):")
    print("-" * 70)
    for i, entity in enumerate(result.entities, 1):
        print(f"{i}. {entity.name}")
        print(f"   Type: {entity.entity_type}")
        print(f"   Relevance: {entity.temporal_relevance}")
        print(f"   Confidence: {entity.confidence:.2%}")
        if entity.attributes:
            print(f"   Attributes: {json.dumps(entity.attributes, indent=2)}")
        print()
    
    # Relationships
    print(f"RELATIONSHIPS ({len(result.relationships)}):")
    print("-" * 70)
    for i, rel in enumerate(result.relationships, 1):
        print(f"{i}. [{rel.confidence:.0%}] {rel.subject} --[{rel.predicate}]--> {rel.obj}")
        print(f"   Temporal: {rel.temporal_type}")
        print()
    
    # Events
    print(f"EVENTS ({len(result.events)}):")
    print("-" * 70)
    for i, event in enumerate(result.events, 1):
        print(f"{i}. [{event.confidence:.0%}] {event.name}")
        print(f"   Type: {event.event_type}")
        print(f"   Participants: {', '.join(event.participants)}")
        print()
    
    # Statistics
    print("="*70)
    print("STATISTICS:")
    print(f"  Entity types: {len(set(e.entity_type for e in result.entities))}")
    print(f"  Relationship predicates: {len(set(r.predicate for r in result.relationships))}")
    print(f"  Event types: {len(set(e.event_type for e in result.events))}")
    print("="*70 + "\n")


def main():
    parser = argparse.ArgumentParser(description="KG Extraction Demo")
    parser.add_argument(
        "--scenario", 
        choices=["matrix", "tech", "medical", "architecture", "all"],
        default="matrix",
        help="Which scenario to process"
    )
    parser.add_argument(
        "--output",
        help="Output JSON file for results"
    )
    
    args = parser.parse_args()
    
    scenarios = ["matrix", "tech", "medical", "architecture"] if args.scenario == "all" else [args.scenario]
    
    all_results = {}
    
    for scenario in scenarios:
        try:
            print(f"\n📄 Loading scenario: {scenario}...")
            text = load_scenario(scenario)
            
            print(f" Running LLM extraction (this may take a moment)...")
            result = extract_kg_from_text(text)
            
            print_results(result, scenario)
            all_results[scenario] = asdict(result)
            
        except Exception as e:
            print(f"❌ Error processing {scenario}: {e}")
            import traceback
            traceback.print_exc()
    
    if args.output and all_results:
        with open(args.output, 'w') as f:
            json.dump(all_results, f, indent=2)
        print(f"💾 Results saved to {args.output}")


if __name__ == "__main__":
    main()
