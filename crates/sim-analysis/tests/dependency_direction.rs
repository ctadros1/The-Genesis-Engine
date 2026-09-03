//! C17.2: the dependency direction that makes "analysis observes, it never
//! instructs" a compile-time property is asserted by the build, not by
//! convention (ADR-0016, ADR-0033).
//!
//! The check reads the crate manifests rather than trusting the workspace
//! graph in anyone's head: `sim-core` must name no analysis crate, and the
//! only dependents of `sim-analysis` may be the CLI and the experiment
//! harness - never the kernel, the persistence adapter, or the server, any
//! of which could carry an analysis output into a tick.

use std::fs;
use std::path::PathBuf;

fn manifest(crate_name: &str) -> String {
    let path: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "..",
        crate_name,
        "Cargo.toml",
    ]
    .iter()
    .collect();
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// The `[dependencies]` section's body, without comments, so a commented
/// suggestion cannot pass as a dependency and a dependency cannot hide
/// behind a comment marker.
fn dependency_lines(text: &str) -> Vec<String> {
    let mut inside = false;
    let mut lines = Vec::new();
    for raw in text.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.starts_with('[') {
            inside = line == "[dependencies]"
                || line == "[build-dependencies]"
                || line.starts_with("[dependencies.")
                || line.starts_with("[build-dependencies.");
            if line.starts_with("[dependencies.") || line.starts_with("[build-dependencies.") {
                lines.push(line.to_owned());
            }
            continue;
        }
        if inside && !line.is_empty() {
            lines.push(line.to_owned());
        }
    }
    lines
}

#[test]
fn sim_core_depends_on_no_analysis_crate() {
    let lines = dependency_lines(&manifest("sim-core"));
    assert!(
        lines.iter().all(|line| !line.contains("sim-analysis")),
        "sim-core names sim-analysis in its dependencies: {lines:?}"
    );
    // And, as the crate's own manifest promises, on nothing in the
    // workspace at all: the kernel is dependency-free by design.
    assert!(
        lines.iter().all(|line| !line.contains("path = ")),
        "sim-core gained a workspace dependency: {lines:?}"
    );
}

#[test]
fn only_the_cli_and_the_harness_may_depend_on_analysis() {
    for forbidden in ["sim-core", "sim-persist", "sim-server", "sim-protocol"] {
        let lines = dependency_lines(&manifest(forbidden));
        assert!(
            lines.iter().all(|line| !line.contains("sim-analysis")),
            "{forbidden} depends on sim-analysis, which would let an analysis output reach \
             a tick: {lines:?}"
        );
    }
    // Dev-dependencies are not exempt: a test-only edge is still an edge
    // that a `cfg(test)` mistake could promote. The kernel's manifest is
    // checked whole.
    assert!(
        !manifest("sim-core").contains("sim-analysis"),
        "sim-core's manifest mentions sim-analysis somewhere"
    );
}
