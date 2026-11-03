use super::*;
use crate::protocol::{JOB_LIMIT, MAX_JOBS_PER_PLUGIN};

#[test]
fn enforces_the_running_job_limit() {
    let (manager, _receiver) = manager("limited", &["/bin/sleep"]);
    let mut jobs = Vec::new();
    for _ in 0..MAX_JOBS_PER_PLUGIN {
        jobs.push(open(&manager, "limited", "/bin/sleep", &["30"]));
    }
    let error = manager
        .open(&plugin("limited"), params("/bin/sleep", &["30"]))
        .unwrap_err();
    assert_eq!(error.code, JOB_LIMIT);
    manager.shutdown();
}
