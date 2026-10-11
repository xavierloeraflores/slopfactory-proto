//! Repeatable, headless measurements of the simulation and render-snapshot path.

use std::{
    error::Error,
    hint::black_box,
    path::PathBuf,
    time::{Duration, Instant},
};

use factory_core::FactorySimulation;
use serde::Serialize;

#[derive(Serialize)]
pub struct BenchmarkReport {
    ok: bool,
    command: &'static str,
    build_label: Option<String>,
    package_version: &'static str,
    debug_assertions: bool,
    ticks: u64,
    samples: u64,
    warmup: u64,
    worlds: Vec<WorldReport>,
}

#[derive(Serialize)]
struct WorldReport {
    world: PathBuf,
    seed: u64,
    start_tick: u64,
    start_state_hash: String,
    end_tick: u64,
    end_state_hash: String,
    tiles: usize,
    resources: usize,
    structures: usize,
    median_ms_per_tick: f64,
    p95_run_ms_per_tick: f64,
    median_simulation_ms_per_tick: f64,
    median_snapshot_ms_per_tick: f64,
    median_ticks_per_second: f64,
    runs: Vec<Sample>,
}

#[derive(Serialize)]
struct Sample {
    total_ms: f64,
    simulation_ms: f64,
    snapshot_ms: f64,
}

pub fn run(
    worlds: Vec<(PathBuf, FactorySimulation)>,
    build_label: Option<String>,
    ticks: u64,
    samples: u64,
    warmup: u64,
) -> Result<BenchmarkReport, Box<dyn Error>> {
    if ticks == 0 || samples == 0 || worlds.is_empty() {
        return Err("bench requires at least one world, tick, and sample".into());
    }
    let mut reports = Vec::with_capacity(worlds.len());
    for (path, original) in worlds {
        let start = original.snapshot();
        for _ in 0..warmup {
            let _ = measure(&original, ticks);
        }
        let mut runs = Vec::new();
        let mut expected_end = None;
        for _ in 0..samples {
            let (sample, end_hash) = measure(&original, ticks);
            if let Some(expected) = &expected_end {
                if expected != &end_hash {
                    return Err(format!(
                        "benchmark replay for {} produced different state hashes",
                        path.display()
                    )
                    .into());
                }
            } else {
                expected_end = Some(end_hash);
            }
            runs.push(sample);
        }
        let per_tick = |value: f64| value / ticks as f64;
        let totals: Vec<_> = runs.iter().map(|run| per_tick(run.total_ms)).collect();
        let simulations: Vec<_> = runs.iter().map(|run| per_tick(run.simulation_ms)).collect();
        let snapshots: Vec<_> = runs.iter().map(|run| per_tick(run.snapshot_ms)).collect();
        let rates: Vec<_> = runs
            .iter()
            .map(|run| 1_000.0 / per_tick(run.total_ms))
            .collect();
        reports.push(WorldReport {
            world: path,
            seed: start.seed,
            start_tick: start.tick,
            start_state_hash: start.state_hash,
            end_tick: start.tick + ticks,
            end_state_hash: expected_end.expect("samples is nonzero"),
            tiles: start.tiles.len(),
            resources: start.resources.len(),
            structures: start.structures.len(),
            median_ms_per_tick: median(&totals),
            p95_run_ms_per_tick: percentile(&totals, 0.95),
            median_simulation_ms_per_tick: median(&simulations),
            median_snapshot_ms_per_tick: median(&snapshots),
            median_ticks_per_second: median(&rates),
            runs,
        });
    }
    Ok(BenchmarkReport {
        ok: true,
        command: "bench",
        build_label,
        package_version: env!("CARGO_PKG_VERSION"),
        debug_assertions: cfg!(debug_assertions),
        ticks,
        samples,
        warmup,
        worlds: reports,
    })
}

impl BenchmarkReport {
    pub fn human_summary(&self) -> String {
        let mut lines = vec![format!(
            "Bench{}: {} ticks × {} measured runs ({} warmup runs)",
            self.build_label
                .as_ref()
                .map_or(String::new(), |label| format!(" [{label}]")),
            self.ticks,
            self.samples,
            self.warmup
        )];
        for report in &self.worlds {
            lines.push(format!(
                "{} | {} structures | median {:.3} ms/tick ({:.0} ticks/s) | simulation {:.3} ms/tick | snapshot {:.3} ms/tick | p95 run {:.3} ms/tick | hash {}",
                report.world.display(),
                report.structures,
                report.median_ms_per_tick,
                report.median_ticks_per_second,
                report.median_simulation_ms_per_tick,
                report.median_snapshot_ms_per_tick,
                report.p95_run_ms_per_tick,
                report.end_state_hash,
            ));
        }
        lines.join("\n")
    }
}

fn measure(original: &FactorySimulation, ticks: u64) -> (Sample, String) {
    // Clone before timing so every run starts from exactly the same save.
    let mut simulation = original.clone();
    let mut simulation_time = Duration::ZERO;
    let mut snapshot_time = Duration::ZERO;
    let total_start = Instant::now();
    for _ in 0..ticks {
        let start = Instant::now();
        black_box(simulation.step());
        simulation_time += start.elapsed();

        let start = Instant::now();
        black_box(simulation.render_snapshot());
        snapshot_time += start.elapsed();
    }
    let total_ms = total_start.elapsed().as_secs_f64() * 1_000.0;
    let end_hash = simulation.state_hash();
    (
        Sample {
            total_ms,
            simulation_ms: simulation_time.as_secs_f64() * 1_000.0,
            snapshot_ms: snapshot_time.as_secs_f64() * 1_000.0,
        },
        end_hash,
    )
}

fn percentile(values: &[f64], fraction: f64) -> f64 {
    let mut ordered = values.to_vec();
    ordered.sort_by(f64::total_cmp);
    let index = ((fraction * ordered.len() as f64).ceil() as usize)
        .saturating_sub(1)
        .min(ordered.len() - 1);
    ordered[index]
}

fn median(values: &[f64]) -> f64 {
    let mut ordered = values.to_vec();
    ordered.sort_by(f64::total_cmp);
    let upper = ordered.len() / 2;
    if ordered.len().is_multiple_of(2) {
        (ordered[upper - 1] + ordered[upper]) / 2.0
    } else {
        ordered[upper]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn benchmarking_replays_without_mutating_the_source() {
        let original = FactorySimulation::new(42);
        let hash = original.state_hash();
        let report = run(
            vec![("sample.json".into(), original.clone())],
            None,
            5,
            2,
            1,
        )
        .unwrap();
        assert_eq!(original.state_hash(), hash);
        assert_eq!(report.worlds[0].start_state_hash, hash);
        assert_eq!(report.worlds[0].end_tick, 5);
        assert_eq!(report.worlds[0].runs.len(), 2);
    }
}
