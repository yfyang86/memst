# Configuration Compatibility Changes

## Summary

Added support for both `api_url` and `base_url` field names in LLM and embedding configuration to maintain compatibility with the Rust backend configuration format.

## Changes Made

### 1. `src/memst_server/config.py`

- **LLMConfig class**: Added `base_url` field and `get_api_url()` method
- **EmbeddingConfig class**: Added `base_url` field and `get_api_url()` method
- **load_config() function**: Added `_load_llm_config()` and `_load_embedding_config()` helpers that map `base_url` to `api_url` when loading from TOML
- **Documentation**: Added comprehensive module docstring explaining configuration options

### 2. `src/memst_server/db.py`

- **get_default_settings_from_config()**: Updated to use `config.llm.get_api_url()` and `config.embedding.get_api_url()` instead of direct field access

### 3. New Files

- **`CONFIG.md`**: Complete configuration documentation
- **`tests/test_config_compatibility.py`**: Unit tests for config compatibility
- **`CHANGES.md`**: This file documenting the changes

## Backward Compatibility

✅ **Fully backward compatible**:
- Existing configs using `api_url` continue to work unchanged
- New configs can use either `api_url` or `base_url`
- The `get_api_url()` method returns the effective URL regardless of which field was used

## Migration Guide

### For users with existing Python-only configs
No changes needed. Continue using `api_url`:
```toml
[llm]
api_url = "http://localhost:8080/v1"
```

### For users sharing config with Rust backend
Use `base_url` to match Rust format:
```toml
[llm]
base_url = "http://localhost:8080/v1"
```

### For users with mixed environments
Both fields work in the same file (though not recommended):
```toml
[llm]
base_url = "http://localhost:8080/v1"  # Takes precedence if both present
api_url = "http://backup:8080/v1"      # Fallback
```

## Testing

Run the compatibility tests:
```bash
cd memst-server
python tests/test_config_compatibility.py
```

Or verify manually by creating a config with `base_url` and checking that the server loads it correctly.
