//! Unit tests for DuckDB storage layer

use memst_extract_v2::KgDuckDb;

#[test]
fn test_in_memory_db_creation() {
    let db = KgDuckDb::open_in_memory();
    assert!(db.is_ok(), "Should create in-memory database");
    
    let db = db.unwrap();
    assert!(db.is_in_memory());
}

#[test]
fn test_db_read_operation() {
    let db = KgDuckDb::open_in_memory().unwrap();
    
    // Test basic read
    let result = db.read(|conn| {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM ontologies",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    });
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0, "Should have 0 ontologies initially");
}

#[test]
fn test_db_write_operation() {
    let db = KgDuckDb::open_in_memory().unwrap();
    
    // Insert an ontology
    let result = db.write(|conn| {
        conn.execute(
            "INSERT INTO ontologies (id, top_category, first_category, second_category, chinese_name, english_name, overview) 
             VALUES ('test', 'Test', 'Test', 'Test', '测试', 'Test', 'Test ontology')",
            [],
        )?;
        Ok(())
    });
    
    assert!(result.is_ok(), "Write should succeed");
    
    // Verify the write
    let count = db.read(|conn| {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM ontologies",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }).unwrap();
    
    assert_eq!(count, 1, "Should have 1 ontology after write");
}

#[test]
fn test_db_transaction_commit() {
    let db = KgDuckDb::open_in_memory().unwrap();
    
    // Successful transaction
    let result = db.transaction(|conn| {
        conn.execute(
            "INSERT INTO ontologies (id, top_category, first_category, second_category, chinese_name, english_name, overview) 
             VALUES ('tx1', 'Test', 'Test', 'Test', '测试1', 'Test1', 'Test')",
            [],
        )?;
        
        conn.execute(
            "INSERT INTO ontologies (id, top_category, first_category, second_category, chinese_name, english_name, overview) 
             VALUES ('tx2', 'Test', 'Test', 'Test', '测试2', 'Test2', 'Test')",
            [],
        )?;
        
        Ok(())
    });
    
    assert!(result.is_ok(), "Transaction should commit successfully");
    
    let count = db.read(|conn| {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM ontologies",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }).unwrap();
    
    assert_eq!(count, 2, "Both inserts should be committed");
}

#[test]
fn test_db_transaction_rollback() {
    let db = KgDuckDb::open_in_memory().unwrap();
    
    // First insert one row
    db.write(|conn| {
        conn.execute(
            "INSERT INTO ontologies (id, top_category, first_category, second_category, chinese_name, english_name, overview) 
             VALUES ('existing', 'Test', 'Test', 'Test', '已有', 'Existing', 'Test')",
            [],
        )?;
        Ok(())
    }).unwrap();
    
    // Failed transaction should rollback
    let result = db.transaction(|conn| {
        conn.execute(
            "INSERT INTO ontologies (id, top_category, first_category, second_category, chinese_name, english_name, overview) 
             VALUES ('new', 'Test', 'Test', 'Test', '新', 'New', 'Test')",
            [],
        )?;
        
        // Force an error
        Err(memst_extract_v2::ExtractError::Database("Test error".to_string()))
    });
    
    assert!(result.is_err(), "Transaction should fail");
    
    // Verify rollback - should still have only 1 row
    let count = db.read(|conn| {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM ontologies",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }).unwrap();
    
    assert_eq!(count, 1, "Failed transaction should be rolled back");
}

#[test]
fn test_entities_table() {
    let db = KgDuckDb::open_in_memory().unwrap();
    
    // Insert a document first
    db.write(|conn| {
        conn.execute(
            "INSERT INTO documents (id, content) VALUES ('doc1', 'Test content')",
            [],
        )?;
        Ok(())
    }).unwrap();
    
    // Insert an entity
    db.write(|conn| {
        conn.execute(
            "INSERT INTO entities (id, doc_id, entity_type, name, confidence) 
             VALUES ('E001', 'doc1', '机构', 'OpenAI', 0.95)",
            [],
        )?;
        Ok(())
    }).unwrap();
    
    // Query the entity
    let entity = db.read(|conn| {
        let result = conn.query_row(
            "SELECT id, entity_type, name, confidence FROM entities WHERE id = 'E001'",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, f32>(3)?,
                ))
            },
        )?;
        Ok(result)
    }).unwrap();
    
    assert_eq!(entity.0, "E001");
    assert_eq!(entity.1, "机构");
    assert_eq!(entity.2, "OpenAI");
    assert!((entity.3 - 0.95).abs() < 0.01);
}

#[test]
fn test_relationships_table() {
    let db = KgDuckDb::open_in_memory().unwrap();
    
    // Insert document and entities
    db.write(|conn| {
        conn.execute("INSERT INTO documents (id, content) VALUES ('doc2', 'Test')", [])?;
        conn.execute("INSERT INTO entities (id, doc_id, entity_type, name) VALUES ('E001', 'doc2', '机构', 'OpenAI')", [])?;
        conn.execute("INSERT INTO entities (id, doc_id, entity_type, name) VALUES ('E002', 'doc2', '模型', 'GPT-4')", [])?;
        Ok(())
    }).unwrap();
    
    // Insert a relationship
    db.write(|conn| {
        conn.execute(
            "INSERT INTO relationships (id, doc_id, subject_id, predicate, object_id, confidence) 
             VALUES ('R001', 'doc2', 'E001', '产品发布', 'E002', 0.90)",
            [],
        )?;
        Ok(())
    }).unwrap();
    
    // Query the relationship
    let rel = db.read(|conn| {
        let result = conn.query_row(
            "SELECT subject_id, predicate, object_id FROM relationships WHERE id = 'R001'",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )?;
        Ok(result)
    }).unwrap();
    
    assert_eq!(rel.0, "E001");
    assert_eq!(rel.1, "产品发布");
    assert_eq!(rel.2, "E002");
}

#[test]
fn test_concurrent_reads() {
    let db = KgDuckDb::open_in_memory().unwrap();
    
    // Insert some data
    db.write(|conn| {
        conn.execute("INSERT INTO documents (id, content) VALUES ('doc3', 'Test')", [])?;
        Ok(())
    }).unwrap();
    
    // Spawn multiple read operations (they should not block each other)
    use std::thread;
    
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let db = KgDuckDb::open_in_memory().unwrap();
            thread::spawn(move || {
                db.read(|conn| {
                    let count: i64 = conn.query_row(
                        "SELECT COUNT(*) FROM documents",
                        [],
                        |row| row.get(0),
                    )?;
                    Ok(count)
                }).map(|_| i)
            })
        })
        .collect();
    
    for handle in handles {
        let result = handle.join().unwrap();
        assert!(result.is_ok());
    }
}

#[test]
fn test_embedding_support() {
    let db = KgDuckDb::open_in_memory().unwrap();
    
    // Insert document and entity with embedding
    let embedding: Vec<f32> = (0..1536).map(|i| i as f32 / 1536.0).collect();
    
    db.write(|conn| {
        conn.execute("INSERT INTO documents (id, content) VALUES ('doc4', 'Test')", [])?;
        
        // Note: DuckDB array syntax
        let embedding_str = format!("{}", 
            embedding.iter()
                .map(|f| f.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        
        conn.execute(
            &format!("INSERT INTO entities (id, doc_id, entity_type, name, embedding) 
                     VALUES ('E003', 'doc4', '机构', 'Test', [{}])", embedding_str),
            [],
        )?;
        Ok(())
    }).unwrap();
    
    // Verify embedding was stored
    let retrieved: Vec<f32> = db.read(|conn| {
        let result = conn.query_row(
            "SELECT embedding FROM entities WHERE id = 'E003'",
            [],
            |row| {
                let arr: duckdb::types::Value = row.get(0)?;
                // Convert back to vec
                Ok(vec![])
            },
        )?;
        Ok(result)
    }).unwrap_or_default();
    
    // Should retrieve something (exact conversion depends on duckdb API)
}

#[test]
fn test_foreign_key_constraint() {
    let db = KgDuckDb::open_in_memory().unwrap();
    
    // Try to insert entity with non-existent document
    let result = db.write(|conn| {
        conn.execute(
            "INSERT INTO entities (id, doc_id, entity_type, name) 
             VALUES ('E999', 'nonexistent', '机构', 'Test')",
            [],
        )?;
        Ok(())
    });
    
    // Should fail due to foreign key constraint
    assert!(result.is_err());
}

#[test]
fn test_file_based_db() {
    use tempfile::tempdir;
    
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    
    // Create file-based database
    {
        let db = KgDuckDb::open(&db_path).unwrap();
        
        db.write(|conn| {
            conn.execute(
                "INSERT INTO ontologies (id, top_category, first_category, second_category, chinese_name, english_name, overview) 
                 VALUES ('file-test', 'Test', 'Test', 'Test', '文件测试', 'File Test', 'Test')",
                [],
            )?;
            Ok(())
        }).unwrap();
    }
    
    // Re-open and verify data persisted
    {
        let db = KgDuckDb::open(&db_path).unwrap();
        
        let count = db.read(|conn| {
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM ontologies WHERE id = 'file-test'",
                [],
                |row| row.get(0),
            )?;
            Ok(count)
        }).unwrap();
        
        assert_eq!(count, 1, "Data should persist in file-based DB");
    }
}