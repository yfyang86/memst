//! Sleep-Time Job Scheduler
//!
//! Manages async consolidation jobs that run during idle periods.

use chrono::{DateTime, Duration, Utc};
use memst_core::objects::{MemoryScope, ObjectId};
use memst_core::types::MemoryTier;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique job ID
pub type JobId = String;

/// Job status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    /// Job is queued
    Queued,
    /// Job is running
    Running,
    /// Job completed successfully
    Completed,
    /// Job failed
    Failed,
    /// Job was cancelled
    Cancelled,
}

/// Trigger for consolidation jobs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsolidationTrigger {
    /// Scheduled time-based trigger
    Scheduled(DateTime<Utc>),
    /// Token budget exceeded
    TokenBudgetExceeded,
    /// Session idle for duration
    SessionIdle(Duration),
    /// Explicit API call
    ExplicitAPI,
}

/// Consolidation job configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationConfig {
    /// Memory scope to consolidate
    pub scope: MemoryScope,
    /// Source tier
    pub source_tier: MemoryTier,
    /// Target tier
    pub target_tier: MemoryTier,
    /// Maximum output tokens for summaries
    pub max_output_tokens: u32,
    /// Consolidation strategy
    pub strategy: ConsolidationStrategy,
    /// Whether to commit to MemRepo
    pub commit_to_repo: bool,
    /// Target branch for commits
    pub branch: Option<String>,
}

/// Consolidation strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsolidationStrategy {
    /// Hierarchical summarization
    HierarchicalSummary,
    /// Topic clustering
    TopicClustering,
    /// Semantic deduplication
    SemanticDeduplication,
    /// Full reconstruction
    FullReconstruction,
}

/// A consolidation job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationJob {
    /// Job ID
    pub id: JobId,
    /// Memory scope
    pub scope: MemoryScope,
    /// Source branch
    pub source_branch: String,
    /// Target branch
    pub target_branch: String,
    /// Job status
    pub status: JobStatus,
    /// Trigger type
    pub trigger: ConsolidationTrigger,
    /// Number of memories input
    pub memories_in: u32,
    /// Number of memories output
    pub memories_out: u32,
    /// Tokens saved
    pub token_saved: i32,
    /// Creation time
    pub created_at: DateTime<Utc>,
    /// Start time
    pub started_at: Option<DateTime<Utc>>,
    /// Finish time
    pub finished_at: Option<DateTime<Utc>>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Result commit hash
    pub result_commit: Option<ObjectId>,
}

impl ConsolidationJob {
    /// Create a new job
    pub fn new(
        scope: MemoryScope,
        source_branch: &str,
        target_branch: &str,
        trigger: ConsolidationTrigger,
    ) -> Self {
        Self {
            id: format!("job-{}", Utc::now().timestamp_millis()),
            scope,
            source_branch: source_branch.to_string(),
            target_branch: target_branch.to_string(),
            status: JobStatus::Queued,
            trigger,
            memories_in: 0,
            memories_out: 0,
            token_saved: 0,
            created_at: Utc::now(),
            started_at: None,
            finished_at: None,
            error: None,
            result_commit: None,
        }
    }

    /// Mark job as started
    pub fn mark_started(&mut self) {
        self.status = JobStatus::Running;
        self.started_at = Some(Utc::now());
    }

    /// Mark job as completed
    pub fn mark_completed(&mut self, memories_in: u32, memories_out: u32, token_saved: i32, result_commit: Option<ObjectId>) {
        self.status = JobStatus::Completed;
        self.memories_in = memories_in;
        self.memories_out = memories_out;
        self.token_saved = token_saved;
        self.finished_at = Some(Utc::now());
        self.result_commit = result_commit;
    }

    /// Mark job as failed
    pub fn mark_failed(&mut self, error: &str) {
        self.status = JobStatus::Failed;
        self.error = Some(error.to_string());
        self.finished_at = Some(Utc::now());
    }

    /// Mark job as cancelled
    pub fn mark_cancelled(&mut self) {
        self.status = JobStatus::Cancelled;
        self.finished_at = Some(Utc::now());
    }

    /// Get duration if job has finished
    pub fn duration(&self) -> Option<Duration> {
        match (self.started_at, self.finished_at) {
            (Some(start), Some(end)) => Some(end - start),
            _ => None,
        }
    }
}

/// Job scheduler for managing consolidation jobs
pub struct JobScheduler {
    jobs: HashMap<JobId, ConsolidationJob>,
    /// Maximum concurrent jobs
    max_concurrent: usize,
    /// Currently running jobs
    running: Vec<JobId>,
    /// Job queue (FIFO)
    queue: Vec<JobId>,
}

impl JobScheduler {
    /// Create a new scheduler
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            jobs: HashMap::new(),
            max_concurrent,
            running: Vec::new(),
            queue: Vec::new(),
        }
    }

    /// Schedule a new job
    pub fn schedule(&mut self, job: ConsolidationJob) -> JobId {
        let id = job.id.clone();
        self.queue.push(id.clone());
        self.jobs.insert(id.clone(), job);
        id
    }

    /// Get next job to run
    pub fn next_job(&mut self) -> Option<&mut ConsolidationJob> {
        // Check if we can start more jobs
        if self.running.len() >= self.max_concurrent {
            return None;
        }

        // Find next queued job
        while let Some(id) = self.queue.first() {
            if let Some(job) = self.jobs.get(id) {
                if job.status == JobStatus::Queued {
                    let id = id.clone();
                    self.queue.remove(0);
                    self.running.push(id.clone());
                    return self.jobs.get_mut(&id);
                }
            }
            self.queue.remove(0);
        }

        None
    }

    /// Mark a job as complete and remove from running
    pub fn complete_job(&mut self, job_id: &JobId) {
        self.running.retain(|id| id != job_id);
    }

    /// Get job by ID
    pub fn get_job(&self, job_id: &JobId) -> Option<&ConsolidationJob> {
        self.jobs.get(job_id)
    }

    /// Get mutable job by ID
    pub fn get_job_mut(&mut self, job_id: &JobId) -> Option<&mut ConsolidationJob> {
        self.jobs.get_mut(job_id)
    }

    /// Cancel a job
    pub fn cancel_job(&mut self, job_id: &JobId) -> bool {
        // Remove from queue if queued
        self.queue.retain(|id| id != job_id);

        // Mark as cancelled if found
        if let Some(job) = self.jobs.get_mut(job_id) {
            if job.status == JobStatus::Queued || job.status == JobStatus::Running {
                job.mark_cancelled();
                self.running.retain(|id| id != job_id);
                return true;
            }
        }
        false
    }

    /// List all jobs
    pub fn list_jobs(&self) -> Vec<&ConsolidationJob> {
        self.jobs.values().collect()
    }

    /// List jobs by status
    pub fn list_jobs_by_status(&self, status: JobStatus) -> Vec<&ConsolidationJob> {
        self.jobs
            .values()
            .filter(|j| j.status == status)
            .collect()
    }

    /// Get queue position for a job
    pub fn queue_position(&self, job_id: &JobId) -> Option<usize> {
        self.queue.iter().position(|id| id == job_id)
    }

    /// Clean up completed/failed/cancelled jobs older than duration
    pub fn cleanup(&mut self, max_age: Duration) -> usize {
        let cutoff = Utc::now() - max_age;
        let to_remove: Vec<_> = self
            .jobs
            .iter()
            .filter(|(_, job)| {
                matches!(job.status, JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled)
                    && job.finished_at.map(|t| t < cutoff).unwrap_or(false)
            })
            .map(|(id, _)| id.clone())
            .collect();

        let count = to_remove.len();
        for id in to_remove {
            self.jobs.remove(&id);
        }
        count
    }

    /// Get scheduler statistics
    pub fn stats(&self) -> SchedulerStats {
        SchedulerStats {
            total_jobs: self.jobs.len(),
            queued: self.queue.len(),
            running: self.running.len(),
            completed: self.list_jobs_by_status(JobStatus::Completed).len(),
            failed: self.list_jobs_by_status(JobStatus::Failed).len(),
        }
    }
}

/// Scheduler statistics
#[derive(Debug, Clone, Copy)]
pub struct SchedulerStats {
    pub total_jobs: usize,
    pub queued: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_lifecycle() {
        let mut job = ConsolidationJob::new(
            MemoryScope::Session("test".to_string()),
            "main",
            "sleep/test",
            ConsolidationTrigger::ExplicitAPI,
        );

        assert_eq!(job.status, JobStatus::Queued);
        assert!(job.id.starts_with("job-"));

        job.mark_started();
        assert_eq!(job.status, JobStatus::Running);
        assert!(job.started_at.is_some());

        job.mark_completed(100, 50, 500, Some(ObjectId::from_content(b"commit")));
        assert_eq!(job.status, JobStatus::Completed);
        assert_eq!(job.memories_in, 100);
        assert_eq!(job.memories_out, 50);
        assert_eq!(job.token_saved, 500);
        assert!(job.finished_at.is_some());
    }

    #[test]
    fn test_scheduler_schedule_and_run() {
        let mut scheduler = JobScheduler::new(2);

        let job1 = ConsolidationJob::new(
            MemoryScope::Session("test1".to_string()),
            "main",
            "sleep/test1",
            ConsolidationTrigger::ExplicitAPI,
        );
        let id1 = scheduler.schedule(job1);

        let job2 = ConsolidationJob::new(
            MemoryScope::Session("test2".to_string()),
            "main",
            "sleep/test2",
            ConsolidationTrigger::ExplicitAPI,
        );
        let id2 = scheduler.schedule(job2);

        assert_eq!(scheduler.stats().queued, 2);

        // Start first job
        let next = scheduler.next_job().unwrap();
        assert_eq!(next.id, id1);
        next.mark_started();

        assert_eq!(scheduler.stats().running, 1);
        assert_eq!(scheduler.stats().queued, 1);

        // Start second job
        let next = scheduler.next_job().unwrap();
        assert_eq!(next.id, id2);

        // Complete first job
        scheduler.complete_job(&id1);
        assert_eq!(scheduler.stats().running, 1);
    }

    #[test]
    fn test_scheduler_cancel() {
        let mut scheduler = JobScheduler::new(1);

        let job = ConsolidationJob::new(
            MemoryScope::Session("test".to_string()),
            "main",
            "sleep/test",
            ConsolidationTrigger::ExplicitAPI,
        );
        let id = scheduler.schedule(job);

        assert!(scheduler.cancel_job(&id));
        assert_eq!(scheduler.get_job(&id).unwrap().status, JobStatus::Cancelled);

        // Can't cancel again
        assert!(!scheduler.cancel_job(&id));
    }

    #[test]
    fn test_scheduler_queue_position() {
        let mut scheduler = JobScheduler::new(1);

        let job1 = ConsolidationJob::new(
            MemoryScope::Session("test1".to_string()),
            "main",
            "sleep/test1",
            ConsolidationTrigger::ExplicitAPI,
        );
        let id1 = scheduler.schedule(job1);

        let job2 = ConsolidationJob::new(
            MemoryScope::Session("test2".to_string()),
            "main",
            "sleep/test2",
            ConsolidationTrigger::ExplicitAPI,
        );
        let id2 = scheduler.schedule(job2);

        assert_eq!(scheduler.queue_position(&id1), Some(0));
        assert_eq!(scheduler.queue_position(&id2), Some(1));
    }
}
