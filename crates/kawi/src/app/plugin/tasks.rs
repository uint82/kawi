use std::collections::{BTreeMap, VecDeque};
use std::path::Path;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use kawi_plugin::{
    RpcError, TaskEvent as ProtocolTaskEvent, TaskExitedEvent, TaskFindingEvent, TaskOutputEvent,
    TaskSeverity, TaskStartResult, TaskState, TaskStatusResult, TaskStream, MAX_TASKS_PER_PLUGIN,
    MAX_TASK_EVENT_QUEUE_BYTES, MAX_TASK_EVENT_QUEUE_ITEMS, MAX_TASK_FINDING_MESSAGE_BYTES,
    MAX_TASK_FINDING_PATH_BYTES, MAX_TASK_LINE_BYTES, TASK_RETENTION_SECS,
};
use kawi_task::{
    Finding, OutputStream, TaskEvent as RunnerEvent, TaskOptions, TaskRunner, TaskSpec,
};

#[derive(Default)]
pub(crate) struct PluginTaskManager {
    jobs: BTreeMap<String, BTreeMap<u64, PluginTaskJob>>,
    next_job: u64,
}

struct PluginTaskJob {
    runner: Option<TaskRunner>,
    events: VecDeque<ProtocolTaskEvent>,
    event_bytes: usize,
    status: TaskStatusResult,
    finished_at: Option<Instant>,
}

impl PluginTaskManager {
    pub(crate) fn start(
        &mut self,
        plugin: &str,
        spec: &TaskSpec,
        root: &Path,
        wake: Sender<()>,
    ) -> Result<TaskStartResult, RpcError> {
        self.sync_plugin(plugin);
        if self.running(plugin) >= MAX_TASKS_PER_PLUGIN {
            return Err(RpcError::task_limit(MAX_TASKS_PER_PLUGIN));
        }
        let runner = TaskRunner::start_with(spec, root, wake, TaskOptions::default())
            .map_err(|error| RpcError::task_start_failed(error.to_string()))?;
        let job = self.next_job;
        self.next_job = self.next_job.saturating_add(1);
        self.jobs
            .entry(plugin.to_string())
            .or_default()
            .insert(job, PluginTaskJob::new(job, &spec.name, runner));
        Ok(TaskStartResult {
            job,
            name: spec.name.clone(),
        })
    }

    pub(crate) fn status(&mut self, plugin: &str, job: u64) -> Result<TaskStatusResult, RpcError> {
        self.sync_job(plugin, job);
        self.job(plugin, job).map(|job| job.status.clone())
    }

    pub(crate) fn poll(
        &mut self,
        plugin: &str,
        job: u64,
    ) -> Result<Vec<ProtocolTaskEvent>, RpcError> {
        self.sync_job(plugin, job);
        let job = self.job_mut(plugin, job)?;
        job.event_bytes = 0;
        Ok(job.events.drain(..).collect())
    }

    pub(crate) fn cancel(&mut self, plugin: &str, job: u64) -> Result<(), RpcError> {
        let job = self.job(plugin, job)?;
        if let Some(runner) = job.runner.as_ref() {
            runner.cancel();
        }
        Ok(())
    }

    pub(crate) fn sync_all(&mut self) {
        for plugin in self.jobs.keys().cloned().collect::<Vec<_>>() {
            self.sync_plugin(&plugin);
        }
        self.prune_finished();
    }

    pub(crate) fn stop_plugin(&mut self, plugin: &str) {
        self.jobs.remove(plugin);
    }

    pub(crate) fn shutdown(&mut self) {
        self.jobs.clear();
    }

    fn sync_plugin(&mut self, plugin: &str) {
        let jobs = self
            .jobs
            .get(plugin)
            .map(|jobs| jobs.keys().copied().collect::<Vec<_>>())
            .unwrap_or_default();
        for job in jobs {
            self.sync_job(plugin, job);
        }
    }

    fn sync_job(&mut self, plugin: &str, job: u64) {
        let Some(entry) = self
            .jobs
            .get_mut(plugin)
            .and_then(|jobs| jobs.get_mut(&job))
        else {
            return;
        };
        while let Some(event) = entry.runner.as_ref().and_then(TaskRunner::try_recv) {
            match event {
                RunnerEvent::Output { stream, line } => {
                    let (line, truncated) = truncate(line, MAX_TASK_LINE_BYTES);
                    entry.push(ProtocolTaskEvent::Output(TaskOutputEvent {
                        job,
                        channel: task_stream(stream),
                        line,
                        truncated,
                    }));
                }
                RunnerEvent::Finding(finding) => {
                    entry.status.findings = entry.status.findings.saturating_add(1);
                    entry.push(ProtocolTaskEvent::Finding(task_finding(job, finding)));
                }
                RunnerEvent::Finished { status, .. } => {
                    entry.status.state = task_state(status.cancelled, status.exit_code);
                    entry.status.exit_code = status.exit_code;
                    entry.status.success = status.success;
                    entry.status.cancelled = status.cancelled;
                    entry.status.duration_ms = duration_ms(status.duration);
                    entry.push(ProtocolTaskEvent::Exited(TaskExitedEvent {
                        job,
                        state: entry.status.state,
                        exit_code: status.exit_code,
                        success: status.success,
                        cancelled: status.cancelled,
                        duration_ms: entry.status.duration_ms,
                    }));
                    entry.runner = None;
                    entry.finished_at = Some(Instant::now());
                }
            }
        }
    }

    fn running(&self, plugin: &str) -> usize {
        self.jobs
            .get(plugin)
            .map(|jobs| jobs.values().filter(|job| job.runner.is_some()).count())
            .unwrap_or_default()
    }

    fn job(&self, plugin: &str, job: u64) -> Result<&PluginTaskJob, RpcError> {
        self.jobs
            .get(plugin)
            .and_then(|jobs| jobs.get(&job))
            .ok_or_else(|| RpcError::task_not_found(job))
    }

    fn job_mut(&mut self, plugin: &str, job: u64) -> Result<&mut PluginTaskJob, RpcError> {
        self.jobs
            .get_mut(plugin)
            .and_then(|jobs| jobs.get_mut(&job))
            .ok_or_else(|| RpcError::task_not_found(job))
    }

    fn prune_finished(&mut self) {
        let retention = Duration::from_secs(TASK_RETENTION_SECS);
        for jobs in self.jobs.values_mut() {
            jobs.retain(|_, job| {
                job.finished_at
                    .is_none_or(|finished_at| finished_at.elapsed() < retention)
            });
        }
        self.jobs.retain(|_, jobs| !jobs.is_empty());
    }
}

impl PluginTaskJob {
    fn new(job: u64, name: &str, runner: TaskRunner) -> Self {
        Self {
            runner: Some(runner),
            events: VecDeque::new(),
            event_bytes: 0,
            status: TaskStatusResult {
                job,
                name: name.to_string(),
                state: TaskState::Running,
                exit_code: None,
                success: false,
                cancelled: false,
                duration_ms: 0,
                findings: 0,
                dropped_events: 0,
            },
            finished_at: None,
        }
    }

    fn push(&mut self, event: ProtocolTaskEvent) {
        let size = event.size();
        while !self.events.is_empty()
            && (self.events.len() >= MAX_TASK_EVENT_QUEUE_ITEMS
                || self.event_bytes.saturating_add(size) > MAX_TASK_EVENT_QUEUE_BYTES)
        {
            let Some(dropped) = self.events.pop_front() else {
                break;
            };
            self.event_bytes = self.event_bytes.saturating_sub(dropped.size());
            self.status.dropped_events = self.status.dropped_events.saturating_add(1);
        }
        self.event_bytes = self.event_bytes.saturating_add(size);
        self.events.push_back(event);
    }
}

fn task_state(cancelled: bool, exit_code: Option<i32>) -> TaskState {
    if cancelled {
        TaskState::Cancelled
    } else if exit_code.is_some() {
        TaskState::Exited
    } else {
        TaskState::Failed
    }
}

fn task_stream(stream: OutputStream) -> TaskStream {
    match stream {
        OutputStream::Stdout => TaskStream::Stdout,
        OutputStream::Stderr => TaskStream::Stderr,
    }
}

fn task_finding(job: u64, finding: Finding) -> TaskFindingEvent {
    let (path, _) = truncate(
        finding.path.to_string_lossy().into_owned(),
        MAX_TASK_FINDING_PATH_BYTES,
    );
    let (message, _) = truncate(finding.message, MAX_TASK_FINDING_MESSAGE_BYTES);
    TaskFindingEvent {
        job,
        path,
        line: finding.line.try_into().unwrap_or(u32::MAX),
        column: finding.column.try_into().unwrap_or(u32::MAX),
        severity: task_severity(finding.severity),
        message,
    }
}

fn task_severity(severity: kawi_task::Severity) -> TaskSeverity {
    match severity {
        kawi_task::Severity::Error => TaskSeverity::Error,
        kawi_task::Severity::Warning => TaskSeverity::Warning,
        kawi_task::Severity::Information => TaskSeverity::Information,
        kawi_task::Severity::Hint => TaskSeverity::Hint,
    }
}

fn duration_ms(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn truncate(mut text: String, max: usize) -> (String, bool) {
    if text.len() <= max {
        return (text, false);
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    (text, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("kawi_plugin_task_{}_{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn spec(name: &str, command: &str, args: &[&str]) -> TaskSpec {
        TaskSpec {
            name: name.to_string(),
            command: command.to_string(),
            args: args.iter().map(|arg| arg.to_string()).collect(),
            cwd: None,
            env: Default::default(),
            parser: kawi_task::ParserSpec::None,
        }
    }

    fn wait(manager: &mut PluginTaskManager, plugin: &str, job: u64) -> TaskStatusResult {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            manager.sync_all();
            let status = manager.status(plugin, job).unwrap();
            if status.state != TaskState::Running {
                return status;
            }
            assert!(Instant::now() < deadline, "task did not finish");
            thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    #[cfg(unix)]
    fn runs_a_task_and_reports_exit_code() {
        let dir = temp_dir("exit");
        let (wake, _) = mpsc::channel();
        let mut manager = PluginTaskManager::default();
        let started = manager
            .start(
                "demo",
                &spec("fail", "sh", &["-c", "printf hello; exit 7"]),
                &dir,
                wake,
            )
            .unwrap();
        let status = wait(&mut manager, "demo", started.job);
        assert_eq!(status.state, TaskState::Exited);
        assert_eq!(status.exit_code, Some(7));
        assert!(!status.success);
        let events = manager.poll("demo", started.job).unwrap();
        assert!(events.iter().any(|event| matches!(
            event,
            ProtocolTaskEvent::Output(TaskOutputEvent { line, .. }) if line == "hello"
        )));
        assert!(events
            .iter()
            .any(|event| matches!(event, ProtocolTaskEvent::Exited(_))));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn reports_findings_and_queues_them() {
        let dir = temp_dir("findings");
        let (wake, _) = mpsc::channel();
        let mut manager = PluginTaskManager::default();
        let mut spec = spec("check", "sh", &["-c", "echo src/a.go:2:3: bad"]);
        spec.parser = kawi_task::ParserSpec::Go;
        let started = manager.start("demo", &spec, &dir, wake).unwrap();
        let status = wait(&mut manager, "demo", started.job);
        assert_eq!(status.findings, 1);
        let events = manager.poll("demo", started.job).unwrap();
        assert!(events.iter().any(|event| matches!(
            event,
            ProtocolTaskEvent::Finding(TaskFindingEvent {
                line: 2,
                column: 3,
                ..
            })
        )));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn cancel_marks_the_job_cancelled() {
        let dir = temp_dir("cancel");
        let (wake, _) = mpsc::channel();
        let mut manager = PluginTaskManager::default();
        let started = manager
            .start(
                "demo",
                &spec("sleep", "sh", &["-c", "sleep 10"]),
                &dir,
                wake,
            )
            .unwrap();
        thread::sleep(Duration::from_millis(20));
        manager.cancel("demo", started.job).unwrap();
        let status = wait(&mut manager, "demo", started.job);
        assert_eq!(status.state, TaskState::Cancelled);
        assert!(status.cancelled);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn finished_tasks_do_not_count_toward_the_limit() {
        let dir = temp_dir("finished");
        let (wake, _) = mpsc::channel();
        let mut manager = PluginTaskManager::default();
        let finished = manager
            .start(
                "demo",
                &spec("done", "sh", &["-c", "true"]),
                &dir,
                wake.clone(),
            )
            .unwrap();
        wait(&mut manager, "demo", finished.job);
        for _ in 0..MAX_TASKS_PER_PLUGIN {
            manager
                .start(
                    "demo",
                    &spec("sleep", "sh", &["-c", "sleep 1"]),
                    &dir,
                    wake.clone(),
                )
                .unwrap();
        }
        manager.stop_plugin("demo");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn enforces_the_running_task_limit() {
        let dir = temp_dir("limit");
        let (wake, _) = mpsc::channel();
        let mut manager = PluginTaskManager::default();
        for _ in 0..MAX_TASKS_PER_PLUGIN {
            manager
                .start(
                    "demo",
                    &spec("sleep", "sh", &["-c", "sleep 1"]),
                    &dir,
                    wake.clone(),
                )
                .unwrap();
        }
        let error = manager
            .start("demo", &spec("sleep", "sh", &["-c", "sleep 1"]), &dir, wake)
            .unwrap_err();
        assert_eq!(error.code, kawi_plugin::TASK_LIMIT);
        manager.stop_plugin("demo");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn truncates_on_a_character_boundary() {
        let (text, truncated) = truncate("abé".to_string(), 3);
        assert_eq!(text, "ab");
        assert!(truncated);
    }
}
