# KG Extraction V2 - Test Suite

## Test Structure

```
tests/
├── test_ontology.rs       # Unit tests for ontology management
├── test_db.rs            # Unit tests for DuckDB storage
├── test_prompt.rs        # Unit tests for prompt engineering
├── test_integration.rs   # Integration tests
├── test_uat.rs           # User Acceptance Tests with real examples
└── fixtures/
    └── test_schema.json  # Test ontology definitions
```

## Running Tests

### All Tests
```bash
cargo test -p memst-extract-v2
```

### Specific Test Categories

#### Unit Tests
```bash
# Ontology tests
cargo test -p memst-extract-v2 --test test_ontology

# Database tests
cargo test -p memst-extract-v2 --test test_db

# Prompt engine tests
cargo test -p memst-extract-v2 --test test_prompt
```

#### Integration Tests
```bash
cargo test -p memst-extract-v2 --test test_integration
```

#### UAT Tests (with real examples)
```bash
cargo test -p memst-extract-v2 --test test_uat -- --nocapture
```

### Specific Test Scenarios

```bash
# AI Technology Announcement UAT
cargo test -p memst-extract-v2 uat_ai_technology_announcement -- --nocapture

# Semiconductor Supply Chain UAT
cargo test -p memst-extract-v2 uat_semiconductor_supply_chain -- --nocapture

# Geopolitical Risk Analysis UAT
cargo test -p memst-extract-v2 uat_geopolitical_risk_analysis -- --nocapture
```

## Test Coverage

### Unit Tests

| Module | Test Count | Coverage |
|--------|-----------|----------|
| Ontology | 15+ | Schema parsing, entity types, relations |
| Database | 12+ | CRUD, transactions, concurrency |
| Prompt | 15+ | Template generation, stage variants |

### Integration Tests

| Scenario | Description |
|----------|-------------|
| End-to-end extraction | Full pipeline from document to KG |
| Multi-domain extraction | Multiple ontologies per document |
| Job tracking | Extraction status monitoring |
| Concurrent processing | Parallel document processing |

### UAT Tests

| ID | Scenario | Domain | Complexity |
|----|----------|--------|------------|
| UAT-001 | AI Technology Announcement | AI + Biotech | Medium |
| UAT-002 | Semiconductor Supply Chain | Semi + Supply Chain | High |
| UAT-003 | Geopolitical Risk Analysis | Geopolitics + Risk | High |
| UAT-004 | M&A Event Extraction | M&A + Investment | Medium |
| UAT-005 | Batch Processing | Multi-domain | Medium |
| UAT-006 | Entity Linking | AI | Medium |

## Test Data

### Sample Documents

The UAT tests use realistic intelligence analysis scenarios:

1. **AlphaFold 3 Announcement** - Biotech/AI crossover
2. **TSMC Arizona Investment** - Supply chain analysis
3. **US-China Chip Controls** - Geopolitical risk
4. **Microsoft-Activision Deal** - M&A tracking
5. **Multi-document Batch** - Tesla, Pfizer, SpaceX news

### Expected Outputs

Each UAT test validates:
- Entity extraction (names, types, confidence)
- Relation extraction (subject-predicate-object)
- Argument extraction (time, value, location)
- Database persistence
- Job tracking

## Continuous Integration

### Pre-commit Checks
```bash
# Run all tests
cargo test -p memst-extract-v2

# Check formatting
cargo fmt -p memst-extract-v2 -- --check

# Run clippy
cargo clippy -p memst-extract-v2 -- -D warnings
```

### Nightly Tests
```bash
# Full UAT suite with output
cargo test -p memst-extract-v2 --test test_uat -- --nocapture

# Stress test with large documents
cargo test -p memst-extract-v2 test_long_content_handling
```

## Adding New Tests

### Unit Test Template
```rust
#[test]
fn test_new_feature() {
    // Arrange
    let input = ...;
    
    // Act
    let result = function_under_test(input);
    
    // Assert
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), expected);
}
```

### UAT Test Template
```rust
#[tokio::test]
async fn uat_new_scenario() {
    let (service, db) = setup_production_service();
    
    let doc = Document {
        id: "uat-xxx".to_string(),
        content: "Realistic content...".to_string(),
        title: Some("Title".to_string()),
        source: Some("Source".to_string()),
        ...
    };
    
    let config = ExtractionConfig {
        ontology_ids: vec!["domain-id".to_string()],
        ...
    };
    
    let results = service.extract(&doc, &config).await;
    
    // Validation assertions
    assert!(...);
}
```

## Performance Benchmarks

### Document Processing Speed
- Small documents (< 1KB): < 100ms
- Medium documents (1-10KB): < 500ms
- Large documents (10-100KB): < 2s

### Database Operations
- Entity insert: ~1ms
- Relation insert: ~1ms
- Query by ID: < 10ms
- Full-text search: < 100ms

## Debugging Failed Tests

### Enable Debug Output
```bash
RUST_LOG=debug cargo test -p memst-extract-v2 -- --nocapture
```

### Inspect Database State
```bash
# Run test that persists data
cargo test -p memst-extract-v2 test_end_to_end_extraction

# Connect with DuckDB CLI
duckdb /tmp/test.db

# Query extracted data
SELECT * FROM entities WHERE doc_id = 'test-doc-001';
```

### Check Prompt Output
```rust
// In test code
let prompt = engine.build_extraction_prompt(...).unwrap();
println!("Generated prompt:\n{}", prompt);
```

## Known Limitations

1. **Mock Extraction**: Current implementation uses mock extraction. Real LLM integration pending.
2. **Entity Linking**: Basic implementation, advanced disambiguation not yet implemented.
3. **Vector Search**: Schema supports embeddings, but similarity search not yet implemented.
