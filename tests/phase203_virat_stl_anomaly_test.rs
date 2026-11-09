//! Phase 203 Integration Test:
//! Quantitative Signal Temporal Logic (Q-STL) Anomaly Detection on VIRAT Video Dataset v2.0.
//!
//! Evaluates the `evaluate_stl_anomaly` MCP tool across formal security invariants:
//! - Loitering & Extended Dwell Time: $\phi_{\text{loiter}} = \square_{[0, T]} (\text{in\_zone}(P) \implies \text{duration} \le 60\text{s})$
//! - Unattended / Abandoned Bag: $\phi_{\text{bag}} = \square (\text{is\_bag}(O) \implies \text{dist}(O, \text{Owner}) \le 3.0\text{m})$
//! - Unauthorized Vehicle Tampering: $\phi_{\text{veh}} = \square (\text{dist}(P, \text{Vehicle}) \ge 1.5\text{m} \lor \text{authorized}(P))$
//! - Sterile Perimeter Breach: $\phi_{\text{breach}} = \square (\text{in\_zone}(P, \text{Sterile}) \implies \text{speed} \le 2.0\text{ m/s})$
//!
//! Grounded in:
//! - Oh et al., "A Large-scale Benchmark Dataset for Event Recognition in Surveillance Video" (VIRAT), CVPR 2011.
//! - Kitware & DARPA VIRAT Video Dataset Release 2.0 Ground Camera Benchmark.
//! - Maler & Nickovic, "Monitoring Temporal Properties of Continuous Signals", FORMATS 2004.
//! - Donze & Maler, "Robust Satisfaction of Temporal Logic over Real-Valued Signals", FORMATS 2010.

use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use core_types::time_travel::{MotionState, TrajectoryWaypoint};
use core_types::{BoundingBox2D, CoreError, NoOpSecurity, Point3D};
use mcp_server::anomaly::AnomalyWatchdog;
use mcp_server::protocol::{ContentBlock, ToolCallResult};
use mcp_server::tools::{get_tools_manifest, ToolDispatcher};
use parking_lot::{Mutex, RwLock};
use serde::Deserialize;
use serde_json::{json, Value};
use spatiotemporal_db::scene_graph::DynamicSceneGraph;
use spatiotemporal_db::{L2QuerySemanticCache, SpatiotemporalDb};
use tempfile::tempdir;

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

fn extract_text_response(result: &ToolCallResult) -> String {
    let mut out = String::new();
    for block in &result.content {
        let ContentBlock::Text { text } = block;
        out.push_str(text);
    }
    out
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ViratBenchmarkScenario {
    scenario_id: String,
    preset: String,
    description: String,
    expected_anomaly: bool,
    expected_violation_ts_ns: Option<u64>,
    waypoints: Vec<ViratWaypointRaw>,
}

#[derive(Debug, Deserialize)]
struct ViratWaypointRaw {
    timestamp_ns: u64,
    entity_id: u64,
    label: String,
    pos: [f32; 3],
    velocity_mps: f32,
    zone_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ViratBenchmarkFile {
    scenarios: Vec<ViratBenchmarkScenario>,
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

#[test]
fn test_phase203_virat_dataset_integrity() {
    let virat_dir = benchmarks_root().join("virat");
    assert!(
        virat_dir.exists(),
        "VIRAT dataset root directory must exist: {:?}",
        virat_dir
    );

    let videos_dir = virat_dir.join("videos_original");
    assert!(
        videos_dir.exists(),
        "VIRAT videos_original directory must exist: {:?}",
        videos_dir
    );

    let video_count = std::fs::read_dir(&videos_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "mp4"))
        .count();

    println!("Detected {video_count} real VIRAT Ground Camera MP4 videos on disk.");
    assert!(
        video_count >= 180,
        "Expected at least 180 official VIRAT MP4 surveillance videos, found {video_count}"
    );

    let benchmark_json = virat_dir.join("virat_stl_benchmark_suite.json");
    assert!(
        benchmark_json.exists(),
        "VIRAT STL benchmark suite JSON must exist: {:?}",
        benchmark_json
    );

    let annotations_dir = virat_dir.join("annotations");
    assert!(
        annotations_dir.exists(),
        "VIRAT annotations directory must exist: {:?}",
        annotations_dir
    );
}

#[test]
fn test_phase203_mcp_evaluate_stl_anomaly_manifest_registration() {
    let manifest = get_tools_manifest();
    let evaluate_tool = manifest
        .iter()
        .find(|t| t.name == "evaluate_stl_anomaly")
        .expect("Tool 'evaluate_stl_anomaly' MUST be registered in MCP manifest");

    assert!(
        evaluate_tool.description.contains("Signal Temporal Logic")
            || evaluate_tool.description.contains("STL"),
        "Tool description must explain formal STL evaluation capabilities"
    );

    let schema_props = evaluate_tool
        .input_schema
        .get("properties")
        .expect("input_schema must have properties");

    assert!(schema_props.get("formula").is_some());
    assert!(schema_props.get("formula_preset").is_some());
    assert!(schema_props.get("lookback_seconds").is_some());
    assert!(schema_props.get("alert_threshold").is_some());
}

#[tokio::test]
async fn test_phase203_stl_quantitative_robustness_loitering() {
    let (dispatcher, scene_graph) = create_test_dispatcher().await;

    let base_ts = 1_700_000_000_000_000_000u64;
    let end_ts = base_ts + 90 * 1_000_000_000;

    // Populate scene graph with loitering trajectory in restricted zone (90 seconds)
    {
        let mut sg = scene_graph.write();
        sg.upsert_entity(101, "person_intruder", Point3D::new(12.0, 5.0, 0.0), base_ts, false);
        let history = sg.entity_history_mut().entry(101).or_default();

        for s in 0..=90 {
            let ts = base_ts + s * 1_000_000_000;
            history.push(TrajectoryWaypoint {
                timestamp_ns: ts,
                stream_id: "cam_parking_gate".to_string(),
                position: Point3D::new(12.0, 5.0, 0.0),
                velocity_mps: Some(0.04),
                motion_state: MotionState::Stationary,
                bounding_box: BoundingBox2D::new(0.2, 0.2, 0.4, 0.8).unwrap(),
                zone_id: Some("zone_restricted_parking".to_string()),
                supported_by_entity: None,
                reference_handle: format!("handle://frame/cam_parking_gate/{ts}"),
            });
        }
    }

    // Call evaluate_stl_anomaly via MCP ToolDispatcher
    let call_args = json!({
        "formula_preset": "loitering",
        "lookback_seconds": 90.0,
        "alert_threshold": 0.0,
        "target_entity_ids": [101]
    });

    let res = dispatcher
        .dispatch("evaluate_stl_anomaly", Some(call_args), end_ts)
        .await
        .expect("evaluate_stl_anomaly tool call must succeed");

    let text = extract_text_response(&res);
    let v: Value = serde_json::from_str(&text).expect("MCP result must be valid JSON");

    assert_eq!(v["status"], "success");
    assert_eq!(
        v["anomaly_detected"], true,
        "Loitering anomaly must be detected when dwell > 60s"
    );
    let rob = v["robustness"].as_f64().expect("robustness must be f64");
    assert!(
        rob < 0.0,
        "Quantitative robustness must be negative on violation, got {rob}"
    );

    let latency_us = v["latency_us"].as_f64().expect("latency_us must be f64");
    println!("Loitering evaluation latency: {latency_us:.1} µs (robustness: {rob:.3})");
    assert!(
        latency_us < 500.0,
        "STL evaluation latency must be < 500 µs, got {latency_us:.1} µs"
    );
}

#[tokio::test]
async fn test_phase203_stl_quantitative_robustness_unattended_bag() {
    let (dispatcher, scene_graph) = create_test_dispatcher().await;

    let base_ts = 1_700_000_000_000_000_000u64;
    let end_ts = base_ts + 40 * 1_000_000_000;

    // Populate person and bag: separate to 5.0m at t=30s
    {
        let mut p_hist = Vec::new();
        let mut b_hist = Vec::new();

        for s in 0..=40 {
            let ts = base_ts + s * 1_000_000_000;
            let (px, bx, b_vel) = if s <= 15 {
                (s as f32 * 0.8, s as f32 * 0.8 + 0.3, 0.8)
            } else {
                // bag stationary at 12.3m, person walks to 12.0 + (s - 15) * 1.5
                (12.0 + (s - 15) as f32 * 1.5, 12.3, 0.0)
            };

            p_hist.push(TrajectoryWaypoint {
                timestamp_ns: ts,
                stream_id: "cam_terminal".to_string(),
                position: Point3D::new(px, 0.0, 0.0),
                velocity_mps: Some(1.5),
                motion_state: MotionState::Moving,
                bounding_box: BoundingBox2D::new(0.1, 0.1, 0.3, 0.8).unwrap(),
                zone_id: Some("airport_lounge".to_string()),
                supported_by_entity: None,
                reference_handle: format!("handle://frame/cam_terminal/{ts}"),
            });

            b_hist.push(TrajectoryWaypoint {
                timestamp_ns: ts,
                stream_id: "cam_terminal".to_string(),
                position: Point3D::new(bx, 0.0, 0.0),
                velocity_mps: Some(b_vel),
                motion_state: if b_vel == 0.0 {
                    MotionState::Stationary
                } else {
                    MotionState::Moving
                },
                bounding_box: BoundingBox2D::new(0.1, 0.1, 0.2, 0.3).unwrap(),
                zone_id: Some("airport_lounge".to_string()),
                supported_by_entity: None,
                reference_handle: format!("handle://frame/cam_terminal/{ts}"),
            });
        }

        let mut sg = scene_graph.write();
        sg.upsert_entity(201, "person_owner", Point3D::new(0.0, 0.0, 0.0), base_ts, false);
        sg.upsert_entity(202, "bag_backpack", Point3D::new(0.3, 0.0, 0.0), base_ts, false);
        sg.entity_history_mut().insert(201, p_hist);
        sg.entity_history_mut().insert(202, b_hist);
    }

    let call_args = json!({
        "formula_preset": "unattended_bag",
        "lookback_seconds": 40.0,
        "target_entity_ids": [201, 202]
    });

    let res = dispatcher
        .dispatch("evaluate_stl_anomaly", Some(call_args), end_ts)
        .await
        .expect("evaluate_stl_anomaly must succeed");

    let text = extract_text_response(&res);
    let v: Value = serde_json::from_str(&text).unwrap();

    assert_eq!(v["status"], "success");
    assert_eq!(v["anomaly_detected"], true);
    let rob = v["robustness"].as_f64().unwrap();
    assert!(rob < 0.0, "Robustness for unattended bag must be negative");
    println!("Unattended bag anomaly confirmed with robustness {rob:.3}");
}

#[tokio::test]
async fn test_phase203_stl_quantitative_robustness_vehicle_tampering() {
    let (dispatcher, scene_graph) = create_test_dispatcher().await;

    let base_ts = 1_700_000_000_000_000_000u64;
    let end_ts = base_ts + 20 * 1_000_000_000;

    {
        let mut v_hist = Vec::new();
        let mut p_hist = Vec::new();

        for s in 0..=20 {
            let ts = base_ts + s * 1_000_000_000;
            let py = if s < 10 {
                15.0 - (s as f32 * 0.9)
            } else {
                10.5 // 0.5m distance from vehicle (tampering)
            };

            v_hist.push(TrajectoryWaypoint {
                timestamp_ns: ts,
                stream_id: "cam_perimeter".to_string(),
                position: Point3D::new(20.0, 10.0, 0.0),
                velocity_mps: Some(0.0),
                motion_state: MotionState::Stationary,
                bounding_box: BoundingBox2D::new(0.3, 0.3, 0.8, 0.7).unwrap(),
                zone_id: Some("perimeter_lot".to_string()),
                supported_by_entity: None,
                reference_handle: format!("handle://frame/cam_perimeter/{ts}"),
            });

            p_hist.push(TrajectoryWaypoint {
                timestamp_ns: ts,
                stream_id: "cam_perimeter".to_string(),
                position: Point3D::new(20.0, py, 0.0),
                velocity_mps: Some(0.9),
                motion_state: MotionState::Moving,
                bounding_box: BoundingBox2D::new(0.3, 0.2, 0.5, 0.8).unwrap(),
                zone_id: Some("perimeter_lot".to_string()),
                supported_by_entity: None,
                reference_handle: format!("handle://frame/cam_perimeter/{ts}"),
            });
        }

        let mut sg = scene_graph.write();
        sg.upsert_entity(301, "vehicle_patrol", Point3D::new(20.0, 10.0, 0.0), base_ts, true);
        sg.upsert_entity(302, "person_trespasser", Point3D::new(20.0, 15.0, 0.0), base_ts, false);
        sg.entity_history_mut().insert(301, v_hist);
        sg.entity_history_mut().insert(302, p_hist);
    }

    let call_args = json!({
        "formula_preset": "unauthorized_vehicle_tampering",
        "lookback_seconds": 20.0,
        "target_entity_ids": [301, 302]
    });

    let res = dispatcher
        .dispatch("evaluate_stl_anomaly", Some(call_args), end_ts)
        .await
        .unwrap();

    let text = extract_text_response(&res);
    let v: Value = serde_json::from_str(&text).unwrap();

    assert_eq!(v["anomaly_detected"], true);
    let rob = v["robustness"].as_f64().unwrap();
    assert!(rob < 0.0);
    println!("Vehicle tampering confirmed with robustness {rob:.3}");
}

#[tokio::test]
async fn test_phase203_stl_quantitative_robustness_perimeter_breach() {
    let (dispatcher, scene_graph) = create_test_dispatcher().await;

    let base_ts = 1_700_000_000_000_000_000u64;
    let end_ts = base_ts + 20 * 1_000_000_000;

    {
        let mut sg = scene_graph.write();
        sg.upsert_entity(401, "person_runner", Point3D::new(0.0, 50.0, 0.0), base_ts, false);
        let p_hist = sg.entity_history_mut().entry(401).or_default();

        for s in 0..=20 {
            let ts = base_ts + s * 1_000_000_000;
            let (speed, x) = if s < 8 {
                (1.1, s as f32 * 1.1)
            } else {
                (3.9, 8.8 + (s - 8) as f32 * 3.9) // Sprint breach at 3.9 m/s
            };

            p_hist.push(TrajectoryWaypoint {
                timestamp_ns: ts,
                stream_id: "cam_fence".to_string(),
                position: Point3D::new(x, 50.0, 0.0),
                velocity_mps: Some(speed),
                motion_state: MotionState::Moving,
                bounding_box: BoundingBox2D::new(0.1, 0.1, 0.3, 0.8).unwrap(),
                zone_id: Some("sterile_perimeter_zone".to_string()),
                supported_by_entity: None,
                reference_handle: format!("handle://frame/cam_fence/{ts}"),
            });
        }
    }

    let call_args = json!({
        "formula_preset": "sterile_perimeter_breach",
        "lookback_seconds": 20.0,
        "target_entity_ids": [401]
    });

    let res = dispatcher
        .dispatch("evaluate_stl_anomaly", Some(call_args), end_ts)
        .await
        .unwrap();

    let text = extract_text_response(&res);
    let v: Value = serde_json::from_str(&text).unwrap();

    assert_eq!(v["anomaly_detected"], true);
    let rob = v["robustness"].as_f64().unwrap();
    assert!(rob < 0.0);
    println!("Perimeter breach confirmed with robustness {rob:.3}");
}

#[tokio::test]
async fn test_phase203_virat_large_scale_quantitative_benchmark() {
    let benchmark_path = benchmarks_root().join("virat/virat_stl_benchmark_suite.json");
    assert!(benchmark_path.exists());

    let file = File::open(&benchmark_path).unwrap();
    let reader = BufReader::new(file);
    let suite: ViratBenchmarkFile = serde_json::from_reader(reader).unwrap();

    println!(
        "\n================================================================================"
    );
    println!(
        " PHASE 203: LARGE-SCALE QUANTITATIVE STL ANOMALY BENCHMARK (VIRAT v2.0 Ground Truth)"
    );
    println!(
        " Total Evaluated Test Scenarios: {}",
        suite.scenarios.len()
    );
    println!(
        "================================================================================\n"
    );

    let mut true_positives = 0;
    let mut false_positives = 0;
    let mut true_negatives = 0;
    let mut false_negatives = 0;

    let mut total_latency_us = 0.0;
    let mut peak_latency_us = 0.0f64;

    for (idx, scenario) in suite.scenarios.iter().enumerate() {
        let (dispatcher, scene_graph) = create_test_dispatcher().await;

        let mut max_ts = 0u64;
        let mut min_ts = u64::MAX;

        // Ingest waypoints through standard scene graph pipeline APIs (upsert_entity + record_waypoint)
        {
            let mut sg = scene_graph.write();
            for wp in &scenario.waypoints {
                if wp.timestamp_ns > max_ts {
                    max_ts = wp.timestamp_ns;
                }
                if wp.timestamp_ns < min_ts {
                    min_ts = wp.timestamp_ns;
                }

                sg.upsert_entity(
                    wp.entity_id,
                    &wp.label,
                    Point3D::new(wp.pos[0], wp.pos[1], wp.pos[2]),
                    wp.timestamp_ns,
                    wp.label.contains("vehicle"),
                );

                sg.record_waypoint(
                    wp.entity_id,
                    TrajectoryWaypoint {
                        timestamp_ns: wp.timestamp_ns,
                        stream_id: "cam_virat_eval".to_string(),
                        position: Point3D::new(wp.pos[0], wp.pos[1], wp.pos[2]),
                        velocity_mps: Some(wp.velocity_mps),
                        motion_state: if wp.velocity_mps < 0.1 {
                            MotionState::Stationary
                        } else {
                            MotionState::Moving
                        },
                        bounding_box: BoundingBox2D::new(0.1, 0.1, 0.3, 0.8).unwrap(),
                        zone_id: wp.zone_id.clone(),
                        supported_by_entity: None,
                        reference_handle: format!("handle://frame/cam_virat/{ts}", ts = wp.timestamp_ns),
                    },
                );
            }
        }

        let lookback_sec = ((max_ts.saturating_sub(min_ts)) as f64 / 1_000_000_000.0) + 5.0;

        let call_args = json!({
            "formula_preset": scenario.preset,
            "lookback_seconds": lookback_sec,
            "alert_threshold": 0.0,
        });

        let t0 = Instant::now();
        let res = dispatcher
            .dispatch("evaluate_stl_anomaly", Some(call_args), max_ts)
            .await
            .expect("MCP tool dispatch must succeed");
        let elapsed_us = t0.elapsed().as_nanos() as f64 / 1000.0;

        total_latency_us += elapsed_us;
        if elapsed_us > peak_latency_us {
            peak_latency_us = elapsed_us;
        }

        let text = extract_text_response(&res);
        let v: Value = serde_json::from_str(&text).expect("MCP result must be valid json");
        let predicted_anomaly = v["anomaly_detected"].as_bool().unwrap_or(false);

        match (scenario.expected_anomaly, predicted_anomaly) {
            (true, true) => true_positives += 1,
            (false, false) => true_negatives += 1,
            (false, true) => {
                false_positives += 1;
                println!("  [MISMATCH FP] {} {} expected=false got=true", scenario.scenario_id, scenario.preset);
            }
            (true, false) => {
                false_negatives += 1;
                println!("  [MISMATCH FN] {} {} expected=true got=false: {}", scenario.scenario_id, scenario.preset, v);
            }
        }

        if (idx + 1) % 10 == 0 || idx == suite.scenarios.len() - 1 {
            println!(
                "  [{:02}/{}] Scenario: {:<14} | Preset: {:<30} | Expected: {:<5} | Got: {:<5} | Latency: {:.1} µs",
                idx + 1,
                suite.scenarios.len(),
                scenario.scenario_id,
                scenario.preset,
                scenario.expected_anomaly,
                predicted_anomaly,
                elapsed_us
            );
        }
    }

    let precision = if true_positives + false_positives > 0 {
        true_positives as f64 / (true_positives + false_positives) as f64
    } else {
        0.0
    };

    let recall = if true_positives + false_negatives > 0 {
        true_positives as f64 / (true_positives + false_negatives) as f64
    } else {
        0.0
    };

    let f1_score = if precision + recall > 0.0 {
        2.0 * (precision * recall) / (precision + recall)
    } else {
        0.0
    };

    let false_alarm_rate = if false_positives + true_negatives > 0 {
        false_positives as f64 / (false_positives + true_negatives) as f64
    } else {
        0.0
    };

    let mean_latency_us = total_latency_us / suite.scenarios.len() as f64;

    println!("\n--------------------------------------------------------------------------------");
    println!(" STATISTICAL METRICS SUMMARY (VIRAT v2.0 Q-STL ANOMALY BENCHMARK)");
    println!("--------------------------------------------------------------------------------");
    println!(" Confusion Matrix:");
    println!("   True Positives  (TP): {:2} | False Positives (FP): {:2}", true_positives, false_positives);
    println!("   False Negatives (FN): {:2} | True Negatives  (TN): {:2}", false_negatives, true_negatives);
    println!(" Formal Certification Metrics:");
    println!("   Precision            : {:.2}% (Target >= 90.0%)", precision * 100.0);
    println!("   Recall               : {:.2}% (Target >= 94.0%)", recall * 100.0);
    println!("   F1-Score             : {:.4}   (Target >= 0.920)", f1_score);
    println!("   False Alarm Rate     : {:.2}%  (Target <= 2.50%)", false_alarm_rate * 100.0);
    println!(" Latency Profile:");
    println!("   Mean MCP Latency     : {:.2} µs (Target < 50 µs in-engine)", mean_latency_us);
    println!("   Peak MCP Latency     : {:.2} µs", peak_latency_us);
    println!("================================================================================\n");

    assert!(
        precision >= 0.90,
        "Precision must be >= 90.0%, got {:.2}%",
        precision * 100.0
    );
    assert!(
        recall >= 0.94,
        "Recall must be >= 94.0%, got {:.2}%",
        recall * 100.0
    );
    assert!(
        f1_score >= 0.92,
        "F1-Score must be >= 0.92, got {:.4}",
        f1_score
    );
    assert!(
        false_alarm_rate <= 0.025,
        "False Alarm Rate must be <= 2.5%, got {:.2}%",
        false_alarm_rate * 100.0
    );
}
