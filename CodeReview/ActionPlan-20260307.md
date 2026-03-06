# Action Plan - Critical & Medium Issues Fix

**Date:** March 7, 2026  
**Review Reference:** [CR-20260307.md](CR-20260307.md)  
**Detailed Roadmap:** [CR-20260307-Plan-R1.md](CR-20260307-Plan-R1.md)

---

## Summary

This document tracks the completion of critical and medium issues identified in code review CR-20260307.

**Status:** Sprint 1 Complete ✅ | Sprint 2 In Progress ⏳

For detailed sprint planning and future work, see [CR-20260307-Plan-R1.md](CR-20260307-Plan-R1.md).

---

## ✅ COMPLETED: Sprint 1 - Critical Fixes

### Issue #1 - Mock/Fake Data in Tests ✅
- Renamed `kg_extraction_integration_tests.rs` → `kg_extraction_mock_tests.rs`
- Created real LLM integration tests with `#[ignore]` attribute

### Issue #2 - Missing LLM Client in Tests ✅
- Created `MockLlmProvider` in `tests/common/mod.rs`
- Implements `LlmProvider` trait for unit testing

### Issue #3 - Hardcoded Default URLs ✅
- Removed `localhost:8080` defaults from production code
- Added environment variable support: `MEMST_LLM_API_URL`, `MEMST_EMBEDDING_API_URL`
- Added `LlmProviderConfig::local()` for explicit local development

### Issue #4 - Panic in Production Code ✅
- Investigation confirmed panic is in `#[cfg(test)]` module - acceptable

### Issue #5 - Unused Test Variables ✅
- Fixed `uat_session_chat_skills_mcp.rs`

### Issue #6 - Empty Default for Message ✅
- Added documentation warning in `memst-core/src/types/mod.rs`

### Issue #7 - Test-Only Re-exports ✅
- Removed from `kg_decay/mod.rs` and `kg_evolve/mod.rs`

### Issue #8 - Relationship Migration (Critical) ✅
- Implemented `migrate_relationships()` in `KnowledgeGraph`
- Integrated into `apply_merge()` in `KgEvolutionEngine`
- All relationships preserved during entity merges
- **Test:** `test_evolution_merge_with_relationships` passing

---

## Test Results Summary

| Test Suite | Tests | Status |
|------------|-------|--------|
| `uat_session_chat_skills_mcp` | 17 | ✅ PASS |
| `uat_skill_execution` | 18 | ✅ PASS |
| `kg_extraction_mock_tests` | 14 | ✅ PASS |
| `kg_evolution_integration_tests` | 9 | ✅ PASS |
| `kg_extraction_real_tests` | 5 | ✅ PASS |
| **Total** | **63** | **✅ ALL PASS** |

**Real LLM Test Results:**
```bash
MEMST_RUN_INTEGRATION_TESTS=1 cargo test -p memst-sleep --test kg_extraction_real_tests
# Results: 5/5 passed (59s)
```

---

## ⏳ Sprint 2 - Error Handling & Actions

**Status:** Not Started  
**Timeline:** March 8-14, 2026  
**Details:** See [CR-20260307-Plan-R1.md#sprint-2](CR-20260307-Plan-R1.md)

### Planned Work

| Priority | Item | Location |
|----------|------|----------|
| High | Create `KgError` enum with thiserror | `memst-sleep/src/error.rs` |
| High | Migrate kg_evolve to standard errors | `kg_evolve/mod.rs` |
| High | Implement SplitEntity action | `kg_evolve/mod.rs:381-382` |
| High | Implement AddRelationship action | `kg_evolve/mod.rs:384-385` |
| Medium | Implement RemoveRelationship action | `kg_evolve/mod.rs:387-388` |
| Medium | Implement UpdateRelationship action | `kg_evolve/mod.rs:390-391` |

---

## Success Criteria - Sprint 1 ✅

- [x] All tests pass without panics
- [x] MockLlmProvider can be used in any test
- [x] Configuration errors have helpful messages
- [x] No hardcoded URLs in production code
- [x] Mock-based tests are clearly labeled
- [x] Unused test variables fixed or used
- [x] Default for Message properly documented
- [x] Test-only re-exports removed
- [x] Relationship migration implemented

## Success Criteria - Sprint 2 (Pending)

- [ ] `KgError` enum created with thiserror
- [ ] All kg_evolve methods use `Result<bool, KgError>`
- [ ] SplitEntity action implemented
- [ ] AddRelationship action implemented
- [ ] RemoveRelationship action implemented
- [ ] UpdateRelationship action implemented
- [ ] All 9 evolution tests pass with new error handling

---

## Commits

| Commit | Description |
|--------|-------------|
| `bc53282` | feat: Implement relationship migration during entity merge |
| `d3c6f24` | docs: Update code review documents with relationship migration status |

---

## Related Documents

- [CR-20260307.md](CR-20260307.md) - Original code review report
- [CR-20260307-Plan-R1.md](CR-20260307-Plan-R1.md) - Detailed remediation plan with sprints
- [DesignReview-20260307.md](DesignReview-20260307.md) - Design review findings
