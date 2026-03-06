//! MemSt Sleep - Async Consolidation Pipeline
//!
//! This crate provides:
//! - Job scheduler for sleep-time tasks
//! - Memory consolidation and clustering
//! - A-MEM-style memory evolution
//! - KG decay and evolution (Phase 15)

pub mod consolidate;
pub mod evolve;
pub mod jobs;
pub mod kg_decay;
pub mod kg_evolve;
pub mod kg_extract;

use jobs::{ConsolidationJob, JobScheduler};

/// Sleep-time manager
pub struct SleepManager {
    scheduler: JobScheduler,
}

impl SleepManager {
    /// Create a new sleep manager
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            scheduler: JobScheduler::new(max_concurrent),
        }
    }

    /// Get the job scheduler
    pub fn scheduler(&self) -> &JobScheduler {
        &self.scheduler
    }

    /// Get the job scheduler (mutable)
    pub fn scheduler_mut(&mut self) -> &mut JobScheduler {
        &mut self.scheduler
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sleep_manager() {
        let manager = SleepManager::new(2);
        assert_eq!(manager.scheduler().stats().total_jobs, 0);
    }
}
