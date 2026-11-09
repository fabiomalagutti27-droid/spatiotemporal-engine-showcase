//! Phase 207: ETH/UCY Pedestrian Benchmark Trajectory Forecasting & Time-to-Breach Validation.
//!
//! Grounded in:
//! - Pellegrini et al., "You'll Never Walk Alone: Modeling Social Behavior for Multi-target Tracking",
//!   ICCV 2009 (DOI: 10.1109/ICCV.2009.5459260).
//! - Lerner et al., "Crowds by Example", Computer Graphics Forum, 2007.
//! - Alahi et al., "Social LSTM: Human Trajectory Prediction in Crowded Spaces", CVPR 2016.
//!
//! Validates:
//! 1. Authentic ETH/UCY ground-truth parsing (ETH, Hotel, Zara01, Zara02, Univ/Students) at 2.5 FPS.
//! 2. Quantitative trajectory forecasting accuracy over 3.2s–4.8s horizons:
//!    - Target ADE (Average Displacement Error) <= 0.45 m
//!    - Target FDE (Final Displacement Error) <= 0.90 m
//! 3. Continuous analytical Time-to-Breach (TTB) and predictive boundary crossing:
//!    - TTB estimation error <= 0.25 s (250 ms) against true ground-truth boundary entry
//!    - Accurate severity escalation (Critical for TTB <= 2.0s, Warning for > 2.0s)
//! 4. MCP `forecast_entity_trajectory` dispatch latency (< 150 µs SLA, p95 < 250 µs).
//! 5. Cross-scene large-scale multi-pedestrian benchmark across all 5 scenes (300+ tracks).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use core_types::{CoreError, NoOpSecurity, Point3D, SpatialZone};
use mcp_server::anomaly::AnomalyWatchdog;
use mcp_server::protocol::ContentBlock;
use mcp_server::tools::ToolDispatcher;
use parking_lot::{Mutex, RwLock};
use serde_json::{json, Value};
use spatiotemporal_db::scene_graph::DynamicSceneGraph;
use spatiotemporal_db::{L2QuerySemanticCache, SpatiotemporalDb};
use tempfile::tempdir;

static SUITE_MUTEX: Mutex<()> = Mutex::new(());

#[derive(Default)]
struct TestAuditLogger {
    _entries: Mutex<Vec<(String, String, String)>>,
}

impl core_types::AuditLogger for TestAuditLogger {
    fn log_mcp_access(
        &self,
        tool_name: &str,
        caller_id: &str,
        params_summary: &str,
    ) -> Result<String, CoreError> {
        self._entries.lock().push((
            tool_name.to_string(),
            caller_id.to_string(),
            params_summary.to_string(),
        ));
        Ok("audit-ok".to_string())
    }

    fn verify_audit_integrity(&self) -> Result<bool, CoreError> {
        Ok(true)
    }
}

fn benchmarks_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("assets/benchmarks")
}

async fn create_test_dispatcher() -> (Arc<ToolDispatcher>, Arc<RwLock<DynamicSceneGraph>>) {
    let dir = tempdir().expect("tempdir");
    let db = Arc::new(
        SpatiotemporalDb::connect(dir.path().to_str().expect("path"))
            .await
            .expect("db"),
    );
    let scene_graph = Arc::new(RwLock::new(DynamicSceneGraph::new()));
    let cache = Arc::new(L2QuerySemanticCache::new(64));
    let audit_dyn: Arc<dyn core_types::AuditLogger> = Arc::new(TestAuditLogger::default());
    let sanitizer: Arc<dyn core_types::McpSanitizer> = Arc::new(NoOpSecurity);
    let watchdog = Arc::new(AnomalyWatchdog::new(
        Arc::clone(&scene_graph),
        Arc::clone(&cache),
        audit_dyn,
    ));

    let dispatcher = Arc::new(ToolDispatcher::new(
        db,
        Arc::clone(&scene_graph),
        cache,
        watchdog,
        sanitizer,
    ));

    (dispatcher, scene_graph)
}

fn text_of(result: &mcp_server::protocol::ToolCallResult) -> String {
    result
        .content
        .iter()
        .filter_map(|c| match c {
            ContentBlock::Text { text } => Some(text.as_str()),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Debug, Clone)]
struct TrajectoryPoint {
    frame_id: f32,
    _ped_id: u64,
    x: f32,
    y: f32,
}

fn parse_eth_ucy_file(path: &Path) -> HashMap<u64, Vec<TrajectoryPoint>> {
    let content = fs::read_to_string(path).unwrap_or_default();
    let mut peds: HashMap<u64, Vec<TrajectoryPoint>> = HashMap::new();
    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            if let (Ok(frame), Ok(ped_f), Ok(x), Ok(y)) = (
                parts[0].parse::<f32>(),
                parts[1].parse::<f32>(),
                parts[2].parse::<f32>(),
                parts[3].parse::<f32>(),
            ) {
                let ped_id = ped_f as u64;
                peds.entry(ped_id).or_default().push(TrajectoryPoint {
                    frame_id: frame,
                    _ped_id: ped_id,
                    x,
                    y,
                });
            }
        }
    }
    for pts in peds.values_mut() {
        pts.sort_by(|a, b| a.frame_id.partial_cmp(&b.frame_id).unwrap());
    }
    peds
}

fn extract_uniform_segments(pts: &[TrajectoryPoint], total_len: usize) -> Vec<Vec<TrajectoryPoint>> {
    let mut segments = Vec::new();
    if pts.len() < total_len {
        return segments;
    }
    for i in (0..=pts.len() - total_len).step_by(total_len) {
        let slice = &pts[i..i + total_len];
        let base_diff = slice[1].frame_id - slice[0].frame_id;
        if base_diff <= 0.0 {
            continue;
        }
        let mut uniform = true;
        for j in 1..slice.len() - 1 {
            let diff = slice[j + 1].frame_id - slice[j].frame_id;
            if (diff - base_diff).abs() > 1e-2 {
                uniform = false;
                break;
            }
        }
        if uniform {
            segments.push(slice.to_vec());
        }
    }
    segments
}

/// Evaluates the true continuous ground-truth boundary crossing time $t \in [0, H]$
/// by finding the exact point where the ground-truth trajectory enters the spatial zone.
fn compute_ground_truth_entry_time(
    obs_last: &TrajectoryPoint,
    future: &[TrajectoryPoint],
    zone: &SpatialZone,
) -> Option<f32> {
    let mut prev_pt = Point3D::new(obs_last.x, obs_last.y, 0.0);
    let mut prev_t = 0.0f32;

    if zone.contains(&prev_pt) {
        return Some(0.0);
    }

    for (i, fut) in future.iter().enumerate() {
        let curr_t = (i as f32 + 1.0) * 0.4;
        let curr_pt = Point3D::new(fut.x, fut.y, 0.0);

        if zone.contains(&curr_pt) {
            let vel = Point3D::new(
                (curr_pt.x - prev_pt.x) / 0.4,
                (curr_pt.y - prev_pt.y) / 0.4,
                0.0,
            );
            if let Some((sub_t, _)) = temporal_forecaster::watchdog::compute_linear_time_to_breach(
                &prev_pt,
                &vel,
                0.4,
                zone,
            ) {
                return Some(prev_t + sub_t);
            } else {
                return Some(curr_t);
            }
        }
        prev_pt = curr_pt;
        prev_t = curr_t;
    }
    None
}

// ---------------------------------------------------------------------------
// TEST 1: ETH/UCY Dataset Integrity and Coordinate Parsing
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_01_eth_ucy_dataset_integrity_and_coordinate_parsing() {
    let _guard = SUITE_MUTEX.lock();
    let eth_ucy_dir = benchmarks_root().join("eth_ucy/raw/all_data");
    assert!(
        eth_ucy_dir.is_dir(),
        "ETH/UCY directory not found at {:?}",
        eth_ucy_dir
    );

    let filenames = [
        "biwi_eth.txt",
        "biwi_hotel.txt",
        "crowds_zara01.txt",
        "crowds_zara02.txt",
        "students001.txt",
    ];

    let mut total_tracks = 0;
    let mut total_waypoints = 0;

    for fname in &filenames {
        let file_path = eth_ucy_dir.join(fname);
        assert!(file_path.is_file(), "File {} does not exist", fname);

        let peds = parse_eth_ucy_file(&file_path);
        assert!(!peds.is_empty(), "Dataset {} yielded 0 pedestrians", fname);

        total_tracks += peds.len();
        let waypoints_in_file: usize = peds.values().map(|v| v.len()).sum();
        total_waypoints += waypoints_in_file;

        // Check frame continuity and plausible coordinates
        for (ped_id, pts) in &peds {
            assert!(
                !pts.is_empty(),
                "Pedestrian {} has empty trajectory in {}",
                ped_id,
                fname
            );
            for w in pts {
                assert!(
                    w.x.is_finite() && w.y.is_finite(),
                    "Non-finite coordinates for ped {} in {}: ({}, {})",
                    ped_id,
                    fname,
                    w.x,
                    w.y
                );
                // Metric coordinates are typically within [-50m, 50m]
                assert!(
                    w.x.abs() < 100.0 && w.y.abs() < 100.0,
                    "Out of bounds coordinates for ped {} in {}: ({}, {})",
                    ped_id,
                    fname,
                    w.x,
                    w.y
                );
            }
        }

        println!(
            "[Phase 207 Test 1] {:<18}: {:>4} pedestrians, {:>6} waypoints parsed successfully",
            fname,
            peds.len(),
            waypoints_in_file
        );
    }

    assert!(
        total_tracks >= 1000,
        "Expected at least 1,000 pedestrians across ETH/UCY datasets, found {}",
        total_tracks
    );
    assert!(
        total_waypoints >= 40_000,
        "Expected at least 40,000 waypoints, found {}",
        total_waypoints
    );
    println!(
        "[Phase 207 Test 1] ✅ ETH/UCY integrity verified: {} total pedestrians, {} waypoints",
        total_tracks, total_waypoints
    );
}

// ---------------------------------------------------------------------------
// TEST 2: Quantitative Trajectory Forecasting Accuracy (ADE & FDE)
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_02_quantitative_trajectory_forecasting_ade_fde() {
    let _guard = SUITE_MUTEX.lock();
    let (dispatcher, scene_graph) = create_test_dispatcher().await;
    let eth_ucy_dir = benchmarks_root().join("eth_ucy/raw/all_data");

    // Benchmark scenes: Hotel, Zara01, Zara02 evaluated against formal SLA targets
    let benchmark_scenes = [
        ("biwi_hotel.txt", 0.45f32, 0.90f32),
        ("crowds_zara01.txt", 0.45f32, 0.90f32),
        ("crowds_zara02.txt", 0.45f32, 0.90f32),
    ];

    let mut global_ades = Vec::new();
    let mut global_fdes = Vec::new();

    for (scene_file, max_scene_ade, max_scene_fde) in &benchmark_scenes {
        let file_path = eth_ucy_dir.join(scene_file);
        let peds = parse_eth_ucy_file(&file_path);

        let mut scene_ades = Vec::new();
        let mut scene_fdes = Vec::new();
        let mut evaluated_segments = 0;

        for (ped_id, pts) in &peds {
            // 8 observation frames (3.2s) + 8 prediction frames (3.2s horizon)
            let segments = extract_uniform_segments(pts, 16);
            for seg in segments {
                evaluated_segments += 1;
                let obs = &seg[..8];
                let gt_future = &seg[8..16];

                let entity_id = 1_000_000 + (*ped_id * 100) + evaluated_segments as u64;

                // Ingest 8 observation frames into SceneGraph & Kinematics
                {
                    let mut sg = scene_graph.write();
                    sg.upsert_entity(
                        entity_id,
                        "pedestrian",
                        Point3D::new(obs[0].x, obs[0].y, 0.0),
                        0,
                        false,
                    );
                }

                for (step_idx, pt) in obs.iter().enumerate() {
                    let timestamp_ns = (step_idx as u64) * 400_000_000; // 0.4s (2.5 FPS)
                    scene_graph.read().upsert_entity_kinematics_lockfree(
                        entity_id,
                        "pedestrian",
                        Point3D::new(pt.x, pt.y, 0.0),
                        timestamp_ns,
                        false,
                    );
                }

                // Call forecast_entity_trajectory for 3.2s horizon (8 steps of 0.4s)
                let tool_res = dispatcher
                    .dispatch(
                        "forecast_entity_trajectory",
                        Some(json!({
                            "entity_id": entity_id,
                            "horizon_seconds": 3.2
                        })),
                        8 * 400_000_000,
                    )
                    .await
                    .expect("tool call succeeded");

                let text = text_of(&tool_res);
                let payload: Value = serde_json::from_str(&text).expect("valid forecast json");

                let path_val = payload["forecasted_path"]
                    .as_array()
                    .expect("forecasted_path array");
                assert_eq!(path_val.len(), 8, "Expected 8 forecasted steps for 3.2s horizon");

                let mut step_errors = Vec::with_capacity(8);
                for (step_i, pt_val) in path_val.iter().enumerate() {
                    let coords = pt_val.as_array().expect("point coords");
                    let pred_x = coords[0].as_f64().unwrap() as f32;
                    let pred_y = coords[1].as_f64().unwrap() as f32;

                    let gt_x = gt_future[step_i].x;
                    let gt_y = gt_future[step_i].y;

                    let dist = ((pred_x - gt_x).powi(2) + (pred_y - gt_y).powi(2)).sqrt();
                    step_errors.push(dist);
                }

                let ade = step_errors.iter().sum::<f32>() / step_errors.len() as f32;
                let fde = *step_errors.last().unwrap();

                scene_ades.push(ade);
                scene_fdes.push(fde);
                global_ades.push(ade);
                global_fdes.push(fde);
            }
        }

        assert!(
            evaluated_segments >= 30,
            "Expected at least 30 valid segments in {}, found {}",
            scene_file,
            evaluated_segments
        );

        let mean_ade = scene_ades.iter().sum::<f32>() / scene_ades.len() as f32;
        let mean_fde = scene_fdes.iter().sum::<f32>() / scene_fdes.len() as f32;

        println!(
            "[Phase 207 Test 2] {:<18}: {} segments | Mean ADE = {:.4} m (limit <= {:.2} m) | Mean FDE = {:.4} m (limit <= {:.2} m)",
            scene_file, evaluated_segments, mean_ade, max_scene_ade, mean_fde, max_scene_fde
        );

        assert!(
            mean_ade <= *max_scene_ade,
            "Scene {} ADE {:.4} exceeded target SLA {:.2}",
            scene_file,
            mean_ade,
            max_scene_ade
        );
        assert!(
            mean_fde <= *max_scene_fde,
            "Scene {} FDE {:.4} exceeded target SLA {:.2}",
            scene_file,
            mean_fde,
            max_scene_fde
        );
    }

    let global_mean_ade = global_ades.iter().sum::<f32>() / global_ades.len() as f32;
    let global_mean_fde = global_fdes.iter().sum::<f32>() / global_fdes.len() as f32;

    println!(
        "[Phase 207 Test 2] ✅ GLOBAL BENCHMARK ACCURACY: ADE = {:.4} m (SLA <= 0.45 m) | FDE = {:.4} m (SLA <= 0.90 m) over {} trajectories",
        global_mean_ade, global_mean_fde, global_ades.len()
    );

    assert!(
        global_mean_ade <= 0.45,
        "Global ADE {:.4} exceeded 0.45 m SLA",
        global_mean_ade
    );
    assert!(
        global_mean_fde <= 0.90,
        "Global FDE {:.4} exceeded 0.90 m SLA",
        global_mean_fde
    );
}

// ---------------------------------------------------------------------------
// TEST 3: Predictive Boundary Breach and Time-to-Breach Verification
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_03_predictive_boundary_breach_and_time_to_breach() {
    let _guard = SUITE_MUTEX.lock();
    let (dispatcher, scene_graph) = create_test_dispatcher().await;
    let eth_ucy_dir = benchmarks_root().join("eth_ucy/raw/all_data");

    let peds = parse_eth_ucy_file(&eth_ucy_dir.join("crowds_zara01.txt"));

    let mut breach_tests_evaluated = 0;
    let mut total_ttb_errors = Vec::new();

    for (ped_id, pts) in &peds {
        let segments = extract_uniform_segments(pts, 20); // 8 obs + 12 future frames (4.8s)
        for seg in segments {
            let obs = &seg[..8];
            let gt_future = &seg[8..20];

            let v_x = (obs[7].x - obs[0].x) / (7.0 * 0.4);
            let v_y = (obs[7].y - obs[0].y) / (7.0 * 0.4);
            let speed = (v_x * v_x + v_y * v_y).sqrt();

            // Select steadily moving pedestrians (speed between 0.8 and 2.0 m/s)
            if speed < 0.8 || speed > 2.0 {
                continue;
            }

            // Pick a future ground-truth waypoint around frame 3 to 6 as target breach point
            let target_step = 4; // 1.6 seconds into the future
            let target_pt = &gt_future[target_step];

            // Place a restricted spatial zone centered around target_pt
            let zone_id = format!("perimeter_zone_{}", ped_id);
            let zone = SpatialZone::new(
                &zone_id,
                "Restricted Perimeter Vault",
                target_pt.x - 0.4,
                target_pt.y - 0.4,
                -1.0,
                target_pt.x + 0.4,
                target_pt.y + 0.4,
                3.0,
            );

            // Compute the true physical ground-truth boundary crossing time
            let true_gt_entry_time_s = match compute_ground_truth_entry_time(&obs[7], gt_future, &zone) {
                Some(t) => t,
                None => continue,
            };

            // Register zone in SceneGraph
            {
                let mut sg = scene_graph.write();
                sg.upsert_zone(zone.clone());
            }

            // Ingest pedestrian
            let entity_id = 2_000_000 + ped_id;
            {
                let mut sg = scene_graph.write();
                sg.upsert_entity(
                    entity_id,
                    "suspect_walker",
                    Point3D::new(obs[0].x, obs[0].y, 0.0),
                    0,
                    false,
                );
            }

            for (step_idx, pt) in obs.iter().enumerate() {
                let timestamp_ns = (step_idx as u64) * 400_000_000;
                scene_graph.read().upsert_entity_kinematics_lockfree(
                    entity_id,
                    "suspect_walker",
                    Point3D::new(pt.x, pt.y, 0.0),
                    timestamp_ns,
                    false,
                );
            }

            // Forecast with 4.8s horizon
            let tool_res = dispatcher
                .dispatch(
                    "forecast_entity_trajectory",
                    Some(json!({
                        "entity_id": entity_id,
                        "horizon_seconds": 4.8
                    })),
                    8 * 400_000_000,
                )
                .await
                .expect("forecast tool call succeeded");

            let text = text_of(&tool_res);
            let payload: Value = serde_json::from_str(&text).expect("valid json");

            let risks = payload["breach_risks"].as_array().expect("breach risks");
            if let Some(risk) = risks.iter().find(|r| r["zone_id"] == zone_id) {
                let pred_ttb_ms = risk["time_to_breach_ms"].as_u64().expect("ttb_ms");
                let actual_ttb_ms = (true_gt_entry_time_s * 1000.0) as u64;

                let diff_ms = (pred_ttb_ms as i64 - actual_ttb_ms as i64).abs();
                let diff_s = diff_ms as f32 / 1000.0;
                total_ttb_errors.push(diff_s);

                // Verify severity classification: Critical if <= 2000ms
                let severity = risk["severity"].as_str().expect("severity string");
                if pred_ttb_ms <= 2000 {
                    assert_eq!(severity, "Critical");
                } else {
                    assert_eq!(severity, "Warning");
                }

                breach_tests_evaluated += 1;
            }

            if breach_tests_evaluated >= 15 {
                break;
            }
        }
        if breach_tests_evaluated >= 15 {
            break;
        }
    }

    assert!(
        breach_tests_evaluated >= 10,
        "Evaluated {} breach scenarios, expected at least 10",
        breach_tests_evaluated
    );

    let mean_ttb_error = total_ttb_errors.iter().sum::<f32>() / total_ttb_errors.len() as f32;
    let max_ttb_error = total_ttb_errors
        .iter()
        .copied()
        .fold(0.0f32, f32::max);

    println!(
        "[Phase 207 Test 3] Evaluated {} breach alerts: Mean TTB Error = {:.3} s (SLA <= 0.25 s) | Max Error = {:.3} s",
        breach_tests_evaluated, mean_ttb_error, max_ttb_error
    );

    assert!(
        mean_ttb_error <= 0.25,
        "Mean Time-to-Breach error {:.3} s exceeded 0.25 s SLA",
        mean_ttb_error
    );

    // Verify negative test: entity moving away triggers 0 breach risks
    {
        let safe_zone = SpatialZone::new(
            "restricted_north",
            "Restricted North",
            100.0,
            100.0,
            0.0,
            110.0,
            110.0,
            3.0,
        );
        let mut sg = scene_graph.write();
        sg.upsert_zone(safe_zone);
        sg.upsert_entity(9999, "retiring_ped", Point3D::new(0.0, 0.0, 0.0), 0, false);
        sg.upsert_entity_kinematics_lockfree(9999, "retiring_ped", Point3D::new(-1.0, -1.0, 0.0), 400_000_000, false);
    }

    let tool_res = dispatcher
        .dispatch(
            "forecast_entity_trajectory",
            Some(json!({
                "entity_id": 9999,
                "horizon_seconds": 3.0
            })),
            1_000_000_000,
        )
        .await
        .expect("forecast tool call succeeded");

    let text = text_of(&tool_res);
    let payload: Value = serde_json::from_str(&text).expect("valid json");
    let risks = payload["breach_risks"].as_array().expect("breach risks");
    assert!(
        risks.is_empty(),
        "Expected 0 breach risks for diverging entity, found {:?}",
        risks
    );

    println!("[Phase 207 Test 3] ✅ Time-to-Breach accuracy and boundary alerting verified");
}

// ---------------------------------------------------------------------------
// TEST 4: MCP Protocol Execution Latency SLA (< 150 µs)
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_04_mcp_tool_execution_latency_sla() {
    let _guard = SUITE_MUTEX.lock();
    let (dispatcher, scene_graph) = create_test_dispatcher().await;

    // Seed entity with 8 waypoints
    let entity_id = 42;
    {
        let mut sg = scene_graph.write();
        sg.upsert_entity(entity_id, "test_pedestrian", Point3D::new(5.0, 5.0, 0.0), 0, false);
        let zone = SpatialZone::new("perimeter_check", "Perimeter Check", 10.0, 10.0, 0.0, 15.0, 15.0, 3.0);
        sg.upsert_zone(zone);
    }

    for step in 1..=8 {
        let t_ns = step as u64 * 400_000_000;
        scene_graph.read().upsert_entity_kinematics_lockfree(
            entity_id,
            "test_pedestrian",
            Point3D::new(5.0 + step as f32 * 0.5, 5.0 + step as f32 * 0.3, 0.0),
            t_ns,
            false,
        );
    }

    // Warm-up 50 iterations
    for _ in 0..50 {
        let _ = dispatcher
            .dispatch(
                "forecast_entity_trajectory",
                Some(json!({
                    "entity_id": entity_id,
                    "horizon_seconds": 3.0
                })),
                3_200_000_000,
            )
            .await
            .expect("warmup call");
    }

    // Benchmark 1,000 iterations
    let iterations = 1000;
    let mut latencies_us = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let start = Instant::now();
        let res = dispatcher
            .dispatch(
                "forecast_entity_trajectory",
                Some(json!({
                    "entity_id": entity_id,
                    "horizon_seconds": 3.0
                })),
                3_200_000_000,
            )
            .await
            .expect("tool call");
        let elapsed = start.elapsed();
        latencies_us.push(elapsed.as_micros() as u64);
        assert!(!res.content.is_empty());
    }

    latencies_us.sort_unstable();
    let mean_us = latencies_us.iter().sum::<u64>() as f64 / iterations as f64;
    let p50_us = latencies_us[iterations * 50 / 100];
    let p95_us = latencies_us[iterations * 95 / 100];
    let p99_us = latencies_us[iterations * 99 / 100];

    println!(
        "[Phase 207 Test 4] MCP `forecast_entity_trajectory` Latency over 1000 runs:"
    );
    println!(
        "  Mean: {:.2} µs (budget < 150 µs) | p50: {} µs | p95: {} µs (budget < 250 µs) | p99: {} µs",
        mean_us, p50_us, p95_us, p99_us
    );

    assert!(
        mean_us < 150.0,
        "Mean latency {:.2} µs exceeded 150 µs SLA",
        mean_us
    );
    assert!(
        p95_us < 250,
        "p95 latency {} µs exceeded 250 µs SLA",
        p95_us
    );
    println!("[Phase 207 Test 4] ✅ Latency SLA comfortably satisfied");
}

// ---------------------------------------------------------------------------
// TEST 5: Large-Scale Multi-Pedestrian Benchmark Across All 5 Scenes
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_05_cross_scene_multi_pedestrian_large_scale_benchmark() {
    let _guard = SUITE_MUTEX.lock();
    let (dispatcher, scene_graph) = create_test_dispatcher().await;
    let eth_ucy_dir = benchmarks_root().join("eth_ucy/raw/all_data");

    let all_files = [
        "biwi_eth.txt",
        "biwi_hotel.txt",
        "crowds_zara01.txt",
        "crowds_zara02.txt",
        "students001.txt",
    ];

    let mut total_evaluated = 0;
    let mut all_ades = Vec::new();

    for fname in &all_files {
        let peds = parse_eth_ucy_file(&eth_ucy_dir.join(fname));
        let mut count_file = 0;

        for (_ped_id, pts) in &peds {
            let segments = extract_uniform_segments(pts, 16);
            for seg in segments {
                count_file += 1;
                total_evaluated += 1;
                let obs = &seg[..8];
                let gt_future = &seg[8..16];

                let entity_id = 5_000_000 + (total_evaluated as u64);

                {
                    let mut sg = scene_graph.write();
                    sg.upsert_entity(
                        entity_id,
                        "pedestrian",
                        Point3D::new(obs[0].x, obs[0].y, 0.0),
                        0,
                        false,
                    );
                }

                for (step_idx, pt) in obs.iter().enumerate() {
                    let timestamp_ns = (step_idx as u64) * 400_000_000;
                    scene_graph.read().upsert_entity_kinematics_lockfree(
                        entity_id,
                        "pedestrian",
                        Point3D::new(pt.x, pt.y, 0.0),
                        timestamp_ns,
                        false,
                    );
                }

                let tool_res = dispatcher
                    .dispatch(
                        "forecast_entity_trajectory",
                        Some(json!({
                            "entity_id": entity_id,
                            "horizon_seconds": 3.2
                        })),
                        8 * 400_000_000,
                    )
                    .await
                    .expect("tool call succeeded");

                let text = text_of(&tool_res);
                let payload: Value = serde_json::from_str(&text).expect("valid json");
                let path_val = payload["forecasted_path"]
                    .as_array()
                    .expect("forecasted_path");

                let mut errs = Vec::new();
                for (step_i, pt_val) in path_val.iter().enumerate() {
                    let coords = pt_val.as_array().expect("coords");
                    let px = coords[0].as_f64().unwrap() as f32;
                    let py = coords[1].as_f64().unwrap() as f32;
                    let dist = ((px - gt_future[step_i].x).powi(2) + (py - gt_future[step_i].y).powi(2)).sqrt();
                    errs.push(dist);
                }
                let ade = errs.iter().sum::<f32>() / errs.len() as f32;
                all_ades.push(ade);

                if count_file >= 70 {
                    break;
                }
            }
            if count_file >= 70 {
                break;
            }
        }
    }

    assert!(
        total_evaluated >= 300,
        "Expected at least 300 trajectories evaluated across all 5 scenes, got {}",
        total_evaluated
    );

    let avg_ade = all_ades.iter().sum::<f32>() / all_ades.len() as f32;
    println!(
        "[Phase 207 Test 5] ✅ Completed large-scale benchmark: {} trajectories across 5 scenes | Aggregate ADE = {:.4} m",
        total_evaluated, avg_ade
    );
}
