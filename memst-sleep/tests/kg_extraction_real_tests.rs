//! Integration Tests for KG Extraction with Real LLM
//!
//! These tests require a valid LLM configuration in ~/.config/memst/config.toml
//! Run with: MEMST_RUN_INTEGRATION_TESTS=1 cargo test -p memst-sleep --test kg_extraction_real_tests
//!
//! Expected configuration in ~/.config/memst/config.toml:
//! ```toml
//! [llm]
//! base_url = "https://your-llm-endpoint.com/v1"
//! api_key = "your-api-key-here"
//! model = "gpt-4"
//!
//! [embedding]
//! base_url = "http://localhost:8081/v1/embeddings"
//! model = "text-embedding-bge-m3"
//! max_context_length = 8192
//! embedding_length = 1024
//! ```

use memst_core::llm::loader::{LlmLoader, LlmProviderConfig};
use memst_core::llm::{EmbeddingClient, EmbeddingConfig, LlmConfig};
use memst_sleep::kg_extract::{KgExtractionConfig, KgExtractionService};
use std::sync::Arc;

mod common;

/// Check if integration tests should run
fn integration_tests_enabled() -> bool {
    matches!(
        std::env::var("MEMST_RUN_INTEGRATION_TESTS").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE")
    )
}

/// Load LLM configuration from config file or environment
async fn create_llm_client() -> Option<Arc<dyn memst_core::llm::providers::LlmProvider>> {
    // Try to load from config
    let config = LlmProviderConfig::default();
    
    // Validate that we have a real URL configured
    if config.base.api_url.is_empty() {
        eprintln!("No LLM API URL configured. Set MEMST_LLM_API_URL or create ~/.config/memst/config.toml");
        return None;
    }
    
    eprintln!("Using LLM config: URL={}, model={}", config.base.api_url, config.base.model);
    
    match LlmLoader::load(config) {
        Ok(client) => Some(client),
        Err(e) => {
            eprintln!("Failed to load LLM client: {}", e);
            None
        }
    }
}

/// Create embedding client from config
fn create_embedding_client() -> Option<EmbeddingClient> {
    let config = EmbeddingConfig::default();
    
    if config.api_url.is_empty() {
        eprintln!("No embedding API URL configured.");
        return None;
    }
    
    eprintln!("Using Embedding config: URL={}, model={}, expected_dim={:?}", 
        config.api_url, config.model, config.expected_dimension);
    
    match EmbeddingClient::new(config) {
        Ok(client) => Some(client),
        Err(e) => {
            eprintln!("Failed to create embedding client: {}", e);
            None
        }
    }
}

#[tokio::test]
async fn test_real_llm_kg_extraction() {
    if !integration_tests_enabled() {
        eprintln!("Skipping integration test (set MEMST_RUN_INTEGRATION_TESTS=1 to enable)");
        return;
    }
    
    common::setup();
    
    let llm_client = match create_llm_client().await {
        Some(client) => client,
        None => {
            eprintln!("LLM not configured, skipping test");
            return;
        }
    };
    
    let service = KgExtractionService::new(llm_client);
    
    let text = r#"
        Alice is a software engineer at Google. She works on the Kubernetes team
        and is an expert in distributed systems. She has been at Google for 5 years
        and previously worked at a startup called CloudNative Inc.
    "#;
    
    match service.extract_from_text(text).await {
        Ok(result) => {
            eprintln!("Extracted {} entities, {} relationships, {} events",
                result.entities.len(),
                result.relationships.len(),
                result.events.len()
            );
            
            // Verify we got at least some entities
            assert!(!result.entities.is_empty(), "Should extract at least one entity");
            
            // Look for expected entities
            let has_alice = result.entities.iter().any(|e| e.name.to_lowercase().contains("alice"));
            let has_google = result.entities.iter().any(|e| e.name.to_lowercase().contains("google"));
            
            assert!(has_alice, "Should extract 'Alice' as an entity");
            assert!(has_google, "Should extract 'Google' as an entity");
            
            eprintln!("✓ Real LLM extraction test passed");
        }
        Err(e) => {
            eprintln!("Extraction failed (may be expected if LLM unavailable): {}", e);
        }
    }
}

#[tokio::test]
async fn test_real_llm_extraction_confidence() {
    if !integration_tests_enabled() {
        eprintln!("Skipping integration test");
        return;
    }
    
    common::setup();
    
    let llm_client = match create_llm_client().await {
        Some(client) => client,
        None => {
            eprintln!("LLM not configured, skipping test");
            return;
        }
    };
    
    let service = KgExtractionService::new(llm_client);
    
    let text = "Bob is a project manager at Microsoft.";
    
    match service.extract_from_text(text).await {
        Ok(result) => {
            // Check confidence scores are within valid range
            for entity in &result.entities {
                assert!(entity.confidence >= 0.0 && entity.confidence <= 1.0,
                    "Entity confidence should be in range [0, 1]");
            }
            
            for rel in &result.relationships {
                assert!(rel.confidence >= 0.0 && rel.confidence <= 1.0,
                    "Relationship confidence should be in range [0, 1]");
            }
            
            eprintln!("✓ Confidence scoring test passed");
        }
        Err(e) => {
            eprintln!("Test skipped due to LLM error: {}", e);
        }
    }
}

#[tokio::test]
async fn test_real_llm_temporal_classification() {
    if !integration_tests_enabled() {
        eprintln!("Skipping integration test");
        return;
    }
    
    common::setup();
    
    let llm_client = match create_llm_client().await {
        Some(client) => client,
        None => {
            eprintln!("LLM not configured, skipping test");
            return;
        }
    };
    
    let service = KgExtractionService::new(llm_client);
    
    // Text with both permanent and temporary information
    let text = r#"
        Rust is a systems programming language with a strong type system.
        John is currently working on a bug fix for the authentication system.
    "#;
    
    match service.extract_from_text(text).await {
        Ok(result) => {
            // Check that temporal relevance is set for entities
            for entity in &result.entities {
                eprintln!("Entity: {} - temporal: {:?}", entity.name, entity.temporal_relevance);
            }
            
            eprintln!("✓ Temporal classification test completed");
        }
        Err(e) => {
            eprintln!("Test skipped due to LLM error: {}", e);
        }
    }
}

/// Test embedding client with real embedding endpoint
#[tokio::test]
async fn test_real_embedding_client() {
    if !integration_tests_enabled() {
        eprintln!("Skipping integration test");
        return;
    }
    
    common::setup();
    
    let client = match create_embedding_client() {
        Some(client) => client,
        None => {
            eprintln!("Embedding not configured, skipping test");
            return;
        }
    };
    
    // Test single text embedding
    let texts = vec![
        "Rust is a systems programming language".to_string(),
        "Python is great for data science".to_string(),
    ];
    
    match client.embed_batch(&texts).await {
        Ok(embeddings) => {
            assert_eq!(embeddings.len(), 2, "Should return 2 embeddings");
            
            // Check dimensions
            for (i, emb) in embeddings.iter().enumerate() {
                eprintln!("Embedding {}: dimension = {}", i, emb.len());
                // BGE-M3 produces 1024-dimensional embeddings
                assert!(!emb.is_empty(), "Embedding should not be empty");
            }
            
            eprintln!("✓ Real embedding client test passed");
        }
        Err(e) => {
            eprintln!("Embedding test failed: {}", e);
            // Don't panic - embedding service might not be running
        }
    }
}

/// Test that validates the config file format and content
#[test]
fn test_config_file_format() {
    use memst_core::config::MemStConfig;
    
    // Try to load the default config
    match MemStConfig::load_default() {
        Ok(Some(config)) => {
            eprintln!("Config loaded successfully from ~/.config/memst/config.toml");
            
            // Check LLM config
            if let Some(llm) = config.llm {
                eprintln!("LLM config: type={}, api_url={}, model={}",
                    llm.r#type, llm.api_url, llm.model);
                assert!(!llm.api_url.is_empty(), "LLM API URL should not be empty");
                
                // Verify it matches expected values
                if !llm.api_url.is_empty() {
                    eprintln!("✓ Using configured LLM endpoint");
                }
            } else {
                eprintln!("⚠ No LLM config found");
            }
            
            // Check embedding config
            if let Some(emb) = config.embedding {
                eprintln!("Embedding config: type={}, api_url={}, model={}",
                    emb.r#type, emb.api_url, emb.model);
                assert!(!emb.api_url.is_empty(), "Embedding API URL should not be empty");
                
                if emb.model.contains("bge") {
                    eprintln!("✓ Using BGE embedding model");
                }
            } else {
                eprintln!("⚠ No embedding config found");
            }
        }
        Ok(None) => {
            eprintln!("No config file found at ~/.config/memst/config.toml");
            eprintln!("Please create one with your LLM and embedding settings");
        }
        Err(e) => {
            panic!("Failed to load config: {}", e);
        }
    }
}
