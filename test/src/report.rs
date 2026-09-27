//! Results as the test standard wants them: one JSON object per test on
//! stdout, a summary line last, the same lines in `/results/results.jsonl`,
//! and an exit code of 0 (all passed), 1 (a test failed) or 2 (could not run).

use serde_json::{json, Map, Value};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::time::Instant;

/// How one test came out.
pub enum Outcome {
    Pass(String),
    Fail(String),
    /// Not applicable here (a feed this node doesn't serve, or serves only
    /// behind a login). Never counted as a pass.
    Skip(String),
    /// The test could not run (no node, no feed answered). Reported as a
    /// failure, and the run exits 2.
    Infra(String),
}

impl From<Result<String, String>> for Outcome {
    fn from(r: Result<String, String>) -> Outcome {
        match r {
            Ok(d) => Outcome::Pass(d),
            Err(d) => Outcome::Fail(d),
        }
    }
}

pub struct Report {
    pass: u32,
    fail: u32,
    skip: u32,
    infra: u32,
    file: Option<File>,
}

impl Report {
    pub fn new(results: &std::path::Path) -> Report {
        // `/results` is the runner's; without it (a hand run) the lines still
        // go to stdout.
        let file = OpenOptions::new().create(true).append(true).open(results.join("results.jsonl")).ok();
        Report { pass: 0, fail: 0, skip: 0, infra: 0, file }
    }

    /// Run one test, timed, and record it.
    pub fn run(&mut self, name: &str, f: impl FnOnce() -> Outcome) {
        let t = Instant::now();
        let outcome = f();
        self.record(name, outcome, t.elapsed().as_millis(), None)
    }

    /// `run` for a test written as `Ok(detail)` / `Err(what went wrong)`.
    pub fn check(&mut self, name: &str, f: impl FnOnce() -> Result<String, String>) {
        self.run(name, || f().into())
    }

    /// Record a result; `extra`'s members are added to the line as they are
    /// (the long suite's per-wave measurements).
    pub fn record(&mut self, name: &str, outcome: Outcome, ms: u128, extra: Option<Map<String, Value>>) {
        let (word, detail) = match outcome {
            Outcome::Pass(d) => {
                self.pass += 1;
                ("pass", d)
            }
            Outcome::Fail(d) => {
                self.fail += 1;
                ("fail", d)
            }
            Outcome::Skip(d) => {
                self.skip += 1;
                ("skip", d)
            }
            Outcome::Infra(d) => {
                self.infra += 1;
                ("fail", format!("could not run: {d}"))
            }
        };
        let mut line = json!({"test": name, "status": word, "ms": ms as u64, "detail": detail});
        if let (Some(extra), Some(obj)) = (extra, line.as_object_mut()) {
            obj.extend(extra);
        }
        self.line(&line.to_string());
    }

    fn line(&mut self, l: &str) {
        println!("{l}");
        if let Some(f) = &mut self.file {
            let _ = writeln!(f, "{l}");
        }
    }

    /// Print the summary and return the exit code. A real failure outranks
    /// an infrastructure one: it is the more useful thing to be told.
    pub fn finish(mut self) -> i32 {
        let s = json!({"summary": {"pass": self.pass, "fail": self.fail + self.infra, "skip": self.skip}});
        self.line(&s.to_string());
        match (self.fail, self.infra) {
            (0, 0) => 0,
            (0, _) => 2,
            _ => 1,
        }
    }
}
