//! stormview's test container (stormcentral `docs/test-standard.md`, #8).
//!
//! stormview is a library: nothing of its own runs on a node. What it does
//! there is be the shape the node's daemons serve their components in, and
//! the shape every UI reads them with. So the suites test the contract of
//! the commit under test two ways — in this container (the documented wire
//! form, defaults, refusals, builders, formatting), and against the feeds
//! the node's daemons serve right now (`GET /api/v1/components`,
//! `/ws/components`), read with this commit's types.
//!
//! - `short`: the documented summary round-trips; every feed on the node
//!   reads.
//! - `medium`: the whole contract, and each feed's integrity, its exact
//!   read-back, and its websocket snapshot.
//! - `long`: overnight waves of readers sized from this pod's allowance,
//!   measured for slowdown and residue across waves.
//!
//! Read-only: nothing is created in the cluster or on the node, and no
//! action a feed offers is ever invoked. The Svelte half has no runtime of
//! its own either; it is exercised by the host apps' builds.

mod contract;
mod env;
mod feeds;
mod http;
mod long;
mod report;
mod suites;

use report::{Outcome, Report};

fn main() {
    let env = env::Env::read();
    let mut r = Report::new(&env.results);
    match env.suite.as_str() {
        "short" => suites::short(&env, &mut r),
        "medium" => suites::medium(&env, &mut r),
        "long" => long::run(&env, &mut r),
        other => r.record("suite", Outcome::Infra(format!("suite {other:?} is not short, medium or long")), 0, None),
    }
    std::process::exit(r.finish());
}
