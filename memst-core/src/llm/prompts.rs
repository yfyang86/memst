//! Prompt Management System
//!
//! Provides template-based prompts for common LLM tasks with structured output formats.
//!
//! ## Supported Tasks
//!
//! - **KnowledgeGraphExtraction**: Extract entities, relationships, and events from text
//! - **Summarization**: Condense text while preserving key information
//! - **Compression**: Aggressively compress information for storage
//! - **QuestionAnswering**: Answer questions based on context
//! - **EntityResolution**: Resolve ambiguous entity references
//! - **FactExtraction**: Extract atomic facts from text
//! - **ContextGeneration**: Generate context for embeddings
//! - **ContradictionDetection**: Identify contradictions in information
//!
//! ## Usage Example
//!
//! ```rust
//! use memst_core::llm::prompts::{PromptManager, TaskType};
//!
//! let prompts = PromptManager::new();
//!
//! // Get KG extraction prompt
//! let kg_prompt = prompts.kg_extraction("Alice works at Google as a software engineer.");
//!
//! // Get summarization prompt
//! let summary_prompt = prompts.summarize("Long text here...", 50, "concise");
//! ```
//!
//! ## Output Formats
//!
//! ### Knowledge Graph Extraction
//! Expected JSON structure:
//! ```json
//! {
//!   "entities": [
//!     {
//!       "name": "Alice",
//!       "type": "person",
//!       "attributes": {"role": "software engineer"},
//!       "confidence": 0.95,
//!       "temporal_relevance": "long_term"
//!     }
//!   ],
//!   "relationships": [
//!     {
//!       "subject": "Alice",
//!       "predicate": "works_at",
//!       "object": "Google",
//!       "confidence": 0.90,
//!       "temporal_type": "long_term"
//!     }
//!   ],
//!   "events": [],
//!   "overall_confidence": 0.92
//! }
//! ```
//!
//! ### Temporal Relevance Values
//! - `permanent`: Core facts, definitions, universal truths (e.g., "Rust is a programming language")
//! - `long_term`: Characters, stable preferences, ongoing projects (e.g., "User works at Google")
//! - `short_term`: Current tasks, temporary situations (e.g., "Working on a bug fix")
//! - `temporary`: Transient events, momentary states (e.g., "Attending today's meeting")
//!
//! ## Configuration
//!
//! Prompts can be customized by modifying templates or creating new ones:
//!
//! ```rust
//! use memst_core::llm::prompts::PromptTemplate;
//! use std::collections::HashMap;
//!
//! let template = PromptTemplate::new(
//!     "custom_task",
//!     "Process this: {input}"
//! ).with_description("My custom task");
//! ```

use std::collections::HashMap;

/// Type of task for prompt selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskType {
    /// Extract knowledge graph from text
    KnowledgeGraphExtraction,
    /// Summarize text
    Summarization,
    /// Compress information
    Compression,
    /// Answer questions
    QuestionAnswering,
    /// Resolve entity ambiguity
    EntityResolution,
    /// Extract facts
    FactExtraction,
    /// Generate embedding context
    ContextGeneration,
    /// Detect contradictions
    ContradictionDetection,
    /// Custom task
    Custom(&'static str),
}

impl std::fmt::Display for TaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskType::KnowledgeGraphExtraction => write!(f, "kg_extraction"),
            TaskType::Summarization => write!(f, "summarization"),
            TaskType::Compression => write!(f, "compression"),
            TaskType::QuestionAnswering => write!(f, "qa"),
            TaskType::EntityResolution => write!(f, "entity_resolution"),
            TaskType::FactExtraction => write!(f, "fact_extraction"),
            TaskType::ContextGeneration => write!(f, "context_generation"),
            TaskType::ContradictionDetection => write!(f, "contradiction_detection"),
            TaskType::Custom(name) => write!(f, "{}", name),
        }
    }
}

/// A prompt template with variable substitution
#[derive(Debug, Clone)]
pub struct PromptTemplate {
    /// Template name
    pub name: String,
    /// Template content with {variable} placeholders
    template: String,
    /// Default variables
    defaults: HashMap<String, String>,
    /// Description of the template
    pub description: String,
}

impl PromptTemplate {
    /// Create a new prompt template
    pub fn new(name: impl Into<String>, template: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            template: template.into(),
            defaults: HashMap::new(),
            description: String::new(),
        }
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set a default variable value
    pub fn with_default(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.defaults.insert(key.into(), value.into());
        self
    }

    /// Render the template with given variables
    pub fn render(&self, variables: &HashMap<String, String>) -> String {
        let mut result = self.template.clone();

        // Start with defaults, then override with provided variables
        let mut merged = self.defaults.clone();
        merged.extend(variables.clone());

        // Replace placeholders
        for (key, value) in merged {
            let placeholder = format!("{{{}}}", key);
            result = result.replace(&placeholder, &value);
        }

        result
    }

    /// Render with a single variable
    pub fn render_with(&self, key: impl Into<String>, value: impl Into<String>) -> String {
        let mut vars = HashMap::new();
        vars.insert(key.into(), value.into());
        self.render(&vars)
    }
}

/// Manages prompt templates for different tasks
#[derive(Debug, Default)]
pub struct PromptManager {
    templates: HashMap<String, PromptTemplate>,
}

impl PromptManager {
    /// Create a new prompt manager with default templates
    pub fn new() -> Self {
        let mut manager = Self {
            templates: HashMap::new(),
        };
        manager.load_defaults();
        manager
    }

    /// Register a template
    pub fn register(&mut self, template: PromptTemplate) {
        self.templates.insert(template.name.clone(), template);
    }

    /// Get a template by name
    pub fn get(&self, name: &str) -> Option<&PromptTemplate> {
        self.templates.get(name)
    }

    /// Render a template
    pub fn render(&self, name: &str, variables: &HashMap<String, String>) -> Option<String> {
        self.get(name).map(|t| t.render(variables))
    }

    /// Load default templates
    fn load_defaults(&mut self) {
        // Knowledge Graph Extraction
        self.register(PromptTemplate::new(
            TaskType::KnowledgeGraphExtraction.to_string(),
            r#"Extract knowledge graph elements from the following text.

Extract:
1. **Entities**: People, organizations, locations, concepts, technologies, projects, conditions, medications, objects
2. **Relationships**: How entities relate to each other (subject-predicate-object)
3. **Events**: Actions, meetings, decisions, accidents, interactions

For each entity, determine temporal relevance:
- "permanent": Core facts, definitions, universal truths
- "long_term": Characters, stable preferences, ongoing projects, long-lived organizations
- "short_term": Current tasks, temporary situations, short projects
- "temporary": Transient events, momentary states, soon-to-change info

Output STRICTLY as JSON with this structure:
{
  "entities": [
    {
      "name": "entity name",
      "type": "person|organization|location|concept|technology|project|condition|medication|object|event",
      "attributes": {"key": "value"},
      "confidence": 0.0-1.0,
      "temporal_relevance": "permanent|long_term|short_term|temporary"
    }
  ],
  "relationships": [
    {
      "subject": "entity name",
      "predicate": "relationship type",
      "object": "entity name",
      "confidence": 0.0-1.0,
      "temporal_type": "permanent|temporary"
    }
  ],
  "events": [
    {
      "name": "event description",
      "type": "meeting|decision|action|accident|interaction|change",
      "participants": ["entity names"],
      "timestamp": "ISO8601 or null",
      "attributes": {},
      "confidence": 0.0-1.0
    }
  ],
  "overall_confidence": 0.0-1.0
}

Text to analyze:
"""
{text}
"""

Respond with ONLY the JSON object, no markdown, no explanations."#,
        )
        .with_description("Extract knowledge graph from text"));

        // Summarization
        self.register(PromptTemplate::new(
            TaskType::Summarization.to_string(),
            r#"Summarize the following text concisely while preserving key information.

Requirements:
- Capture main points and key details
- Maintain factual accuracy
- Preserve names, dates, and important numbers
- Length: approximately {max_length} words or less
- Style: {style}

Text to summarize:
"""
{text}
"""

Summary:"#,
        )
        .with_description("Summarize text")
        .with_default("max_length", "100")
        .with_default("style", "concise"));

        // Compression
        self.register(PromptTemplate::new(
            TaskType::Compression.to_string(),
            r#"Compress the following information into a condensed form.

Compression requirements:
- Retain all critical facts and relationships
- Remove redundant or obvious information
- Use abbreviations and shorthand where appropriate
- Preserve entity names and key attributes
- Target compression ratio: {compression_ratio}x

Original text:
"""
{text}
"""

Compressed form (compact but complete):"#,
        )
        .with_description("Compress information")
        .with_default("compression_ratio", "3"));

        // Question Answering
        self.register(PromptTemplate::new(
            TaskType::QuestionAnswering.to_string(),
            r#"Answer the following question based on the provided context.

Context:
"""
{context}
"""

Question: {question}

Answer concisely and accurately. If the answer is not in the context, say "I don't have enough information to answer this question."

Answer:"#,
        )
        .with_description("Answer questions from context"));

        // Entity Resolution
        self.register(PromptTemplate::new(
            TaskType::EntityResolution.to_string(),
            r#"Determine if these two entities refer to the same real-world entity.

Entity 1:
- Name: {name1}
- Type: {type1}
- Context: {context1}

Entity 2:
- Name: {name2}
- Type: {type2}
- Context: {context2}

Analyze the similarity and respond with JSON:
{
  "is_same_entity": true|false,
  "confidence": 0.0-1.0,
  "reasoning": "explanation",
  "suggested_merged_name": "best name to use if same"
}"#,
        )
        .with_description("Resolve if two entities are the same"));

        // Fact Extraction
        self.register(PromptTemplate::new(
            TaskType::FactExtraction.to_string(),
            r#"Extract factual statements from the following text.

Focus on:
- Personal preferences and likes/dislikes
- Important personal details (names, relationships, dates)
- Plans, goals, and intentions
- Technical facts and knowledge

Output as JSON array:
{
  "facts": [
    {
      "content": "the fact statement",
      "category": "preference|personal|plan|technical|other",
      "confidence": 0.0-1.0,
      "entities_involved": ["entity names"]
    }
  ]
}

Text:
"""
{text}
""""#,
        )
        .with_description("Extract factual statements"));

        // Context Generation
        self.register(PromptTemplate::new(
            TaskType::ContextGeneration.to_string(),
            r#"Generate a context paragraph that would be relevant for answering questions about the following topics.

Topics: {topics}
Key entities: {entities}
Purpose: {purpose}

Generate 2-3 paragraphs of relevant context that:
- Explains relationships between entities
- Provides background information
- Includes relevant facts and details
- Is written in a clear, informative style

Context:"#,
        )
        .with_description("Generate context for queries")
        .with_default("purpose", "general knowledge"));

        // Contradiction Detection
        self.register(PromptTemplate::new(
            TaskType::ContradictionDetection.to_string(),
            r#"Analyze these two statements for contradictions.

Statement 1: {statement1}
Statement 2: {statement2}

Respond with JSON:
{
  "has_contradiction": true|false,
  "confidence": 0.0-1.0,
  "explanation": "why they do or don't contradict",
  "resolution": "how to resolve if contradictory"
}"#,
        )
        .with_description("Detect contradictions between statements"));
    }

    /// Get the KG extraction prompt
    pub fn kg_extraction(&self, text: &str) -> String {
        self.get(&TaskType::KnowledgeGraphExtraction.to_string())
            .expect("KG extraction template not found")
            .render_with("text", text)
    }

    /// Get summarization prompt
    pub fn summarize(&self, text: &str, max_words: u32, style: &str) -> String {
        let template = self.get(&TaskType::Summarization.to_string())
            .expect("Summarization template not found");
        
        let mut vars = HashMap::new();
        vars.insert("text".to_string(), text.to_string());
        vars.insert("max_length".to_string(), max_words.to_string());
        vars.insert("style".to_string(), style.to_string());
        
        template.render(&vars)
    }

    /// Get compression prompt
    pub fn compress(&self, text: &str, ratio: u32) -> String {
        let template = self.get(&TaskType::Compression.to_string())
            .expect("Compression template not found");
        
        let mut vars = HashMap::new();
        vars.insert("text".to_string(), text.to_string());
        vars.insert("compression_ratio".to_string(), ratio.to_string());
        
        template.render(&vars)
    }

    /// Get QA prompt
    pub fn answer(&self, context: &str, question: &str) -> String {
        let template = self.get(&TaskType::QuestionAnswering.to_string())
            .expect("QA template not found");
        
        let mut vars = HashMap::new();
        vars.insert("context".to_string(), context.to_string());
        vars.insert("question".to_string(), question.to_string());
        
        template.render(&vars)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_template_render() {
        let template = PromptTemplate::new(
            "test",
            "Hello {name}, you are {age} years old."
        )
        .with_default("age", "25");

        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "Alice".to_string());
        
        let result = template.render(&vars);
        assert_eq!(result, "Hello Alice, you are 25 years old.");
    }

    #[test]
    fn test_prompt_manager_defaults() {
        let manager = PromptManager::new();
        
        // Check all default templates are loaded
        assert!(manager.get("kg_extraction").is_some());
        assert!(manager.get("summarization").is_some());
        assert!(manager.get("compression").is_some());
        assert!(manager.get("qa").is_some());
        assert!(manager.get("entity_resolution").is_some());
        assert!(manager.get("fact_extraction").is_some());
        assert!(manager.get("context_generation").is_some());
        assert!(manager.get("contradiction_detection").is_some());
    }

    #[test]
    fn test_kg_extraction_prompt() {
        let manager = PromptManager::new();
        let text = "Neo meets The Oracle.";
        let prompt = manager.kg_extraction(text);
        
        assert!(prompt.contains("Extract knowledge graph"));
        assert!(prompt.contains(text));
        assert!(prompt.contains("person|organization|location"));
    }

    #[test]
    fn test_summarize_prompt() {
        let manager = PromptManager::new();
        let text = "A long text to summarize.";
        let prompt = manager.summarize(text, 50, "formal");
        
        assert!(prompt.contains("Summarize the following"));
        assert!(prompt.contains(text));
        assert!(prompt.contains("50"));
        assert!(prompt.contains("formal"));
    }
}
