# KG Extraction Test Scenarios

This directory contains test scenarios for Knowledge Graph extraction across different domains.

## Scenarios

| File | Domain | Description |
|------|--------|-------------|
| `01_the_matrix.md` | Sci-Fi Movie | Character dialogue, prophecy, fate |
| `02_tech_meeting.md` | Business Meeting | Project planning, decisions, action items |
| `03_medical_consultation.md` | Healthcare | Symptoms, diagnoses, treatments |
| `04_software_architecture.md` | Technical Design | System components, relationships |
| `05_research_paper.md` | Academic | Concepts, citations, contributions |

## Usage

```rust
use memst_sleep::kg_extract::KgExtractionService;
use memst_core::llm::LlmClient;

let llm_client = LlmClient::with_defaults();
let service = KgExtractionService::new(llm_client);

// Read scenario
let text = std::fs::read_to_string("demo/kg_test_scenarios/01_the_matrix.md")?;

// Extract KG
let result = service.extract_from_text(&text).await?;

println!("Entities: {}", result.entities.len());
println!("Relationships: {}", result.relationships.len());
println!("Events: {}", result.events.len());
```

## Expected Extraction Quality

### Characters (Person entities)
- Should extract: Neo, The Oracle, Morpheus
- Should classify as: `person`
- Temporal relevance: `long_term` (recurring characters)

### Concepts
- Should extract: "The One", prophecy, fate, choice
- Should classify as: `concept`
- Temporal relevance: `permanent` (core story concepts)

### Relationships
- Neo meets The Oracle
- Morpheus believes in Neo
- The Oracle predicts Neo's choice

### Events
- Neo breaks the vase
- The Oracle gives Neo a cookie
- Morpheus's impending sacrifice
