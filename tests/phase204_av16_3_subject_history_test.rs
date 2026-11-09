use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use audio_spatial_bridge::raycast_ssl::{AcousticRaycaster, TdoaMeasurement};
use audio_spatial_bridge::spatial::{AcousticSpatialMapper, MicrophoneNode, SPEED_OF_SOUND_MPS};
use core_types::audio::AcousticEventKind;
use core_types::spatial::SpatialZone;
use core_types::time_travel::{MotionState, TrajectoryWaypoint};
use core_types::{BoundingBox2D, CoreError, NoOpSecurity, Point3D};
use mcp_server::anomaly::AnomalyWatchdog;
use mcp_server::protocol::{ContentBlock, ToolCallResult};
use mcp_server::tools::{get_tools_manifest, ToolDispatcher};
use parking_lot::{Mutex, RwLock};
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

/// Builds the official Idiap AV16.3 16-microphone 3D spatial layout.
/// Two 8-microphone uniform circular arrays (radius 0.1m, centers at (0, 0.4, 0) and (0, -0.4, 0)).
fn build_av16_3_microphone_mapper() -> (AcousticSpatialMapper, Vec<Point3D>) {
    let mut mapper = AcousticSpatialMapper::new();
    let zone = SpatialZone::new("smart_room_idiap", "Idiap Smart Meeting Room", -4.1, -1.8, -0.84, 8.2, 3.6, 2.4);

    let mut mic_positions = Vec::with_capacity(16);

    for i in 0..16 {
        let array_idx = if i < 8 { 1 } else { 2 };
        let mic_idx_in_array = (i % 8) + 1;
        let channel_id = format!("array{}_mic{}", array_idx, mic_idx_in_array);

        let angle = std::f32::consts::PI * (-1.0 + 2.0 * ((i % 8) as f32) / 8.0);
        let y_center = if i < 8 { 0.4 } else { -0.4 };
        let pos = Point3D::new(
            0.1 * angle.cos(),
            y_center + 0.1 * angle.sin(),
            0.0,
        );

        mic_positions.push(pos);
        mapper.register_microphone(MicrophoneNode::new(
            channel_id,
            pos,
            zone.clone(),
            "omnidirectional",
        ));
    }

    (mapper, mic_positions)
}

#[derive(Debug, Clone)]
struct GroundTruthMouthSample {
    timestamp_s: f64,
    _keyframe_flag: u8,
    pos: Point3D,
}

fn load_av16_3_mouth_ground_truth(path: &PathBuf) -> Vec<GroundTruthMouthSample> {
    assert!(path.exists(), "Ground truth file must exist at {:?}", path);
    let file = File::open(path).expect("open 3dmouthgt");
    let reader = BufReader::new(file);

    let mut samples = Vec::new();
    for line_res in reader.lines() {
        let line = line_res.expect("read line");
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            continue;
        }

        let ts: f64 = parts[0].parse().expect("timestamp parse");
        let flag: u8 = parts[1].parse().expect("flag parse");
        let x: f32 = parts[2].parse().expect("x parse");
        let y: f32 = parts[3].parse().expect("y parse");
        let z: f32 = parts[4].parse().expect("z parse");

        samples.push(GroundTruthMouthSample {
            timestamp_s: ts,
            _keyframe_flag: flag,
            pos: Point3D::new(x, y, z),
        });
    }

    samples
}

#[tokio::test]
async fn test_phase204_av16_3_dataset_integrity() {
    let av_dir = benchmarks_root().join("av16_3");
    let seq18_p1 = av_dir.join("seq18/person1.3dmouthgt");
    let seq18_p2 = av_dir.join("seq18/person2.3dmouthgt");
    let seq11_p1 = av_dir.join("seq11/person1.3dmouthgt");

    assert!(seq18_p1.exists(), "seq18 person1 3dmouthgt missing");
    assert!(seq18_p2.exists(), "seq18 person2 3dmouthgt missing");
    assert!(seq11_p1.exists(), "seq11 person1 3dmouthgt missing");

    let p1_samples = load_av16_3_mouth_ground_truth(&seq18_p1);
    let p2_samples = load_av16_3_mouth_ground_truth(&seq18_p2);
    let s11_samples = load_av16_3_mouth_ground_truth(&seq11_p1);

    assert!(p1_samples.len() >= 1000, "seq18 person1 must contain > 1000 annotated frames (got {})", p1_samples.len());
    assert!(p2_samples.len() >= 1000, "seq18 person2 must contain > 1000 annotated frames (got {})", p2_samples.len());
    assert!(s11_samples.len() >= 400, "seq11 person1 must contain > 400 annotated frames (got {})", s11_samples.len());

    // Sanity-check room coordinates: room is 8.2m x 3.6m x 2.4m, centered at origin
    for s in &p1_samples {
        assert!(s.pos.x >= -4.5 && s.pos.x <= 4.5, "X out of bounds: {}", s.pos.x);
        assert!(s.pos.y >= -2.0 && s.pos.y <= 2.0, "Y out of bounds: {}", s.pos.y);
        assert!(s.pos.z >= -1.0 && s.pos.z <= 2.5, "Z out of bounds: {}", s.pos.z);
    }

    println!(
        "Verified official AV16.3 Ground Truth: seq18 p1 ({} frames), seq18 p2 ({} frames), seq11 p1 ({} frames).",
        p1_samples.len(),
        p2_samples.len(),
        s11_samples.len()
    );
}

#[tokio::test]
async fn test_phase204_mcp_locate_subject_history_manifest_registration() {
    let manifest = get_tools_manifest();
    let tool = manifest
        .iter()
        .find(|t| t.name == "locate_subject_history")
        .expect("Tool 'locate_subject_history' must be registered in MCP manifest");

    assert!(
        tool.description.contains("Reconstruct"),
        "Description must describe spatiotemporal history reconstruction"
    );

    let props = tool.input_schema["properties"].as_object().unwrap();
    assert!(props.contains_key("target_id"));
    assert!(props.contains_key("label"));
    assert!(props.contains_key("start_time_s"));
    assert!(props.contains_key("end_time_s"));
    assert!(props.contains_key("include_waypoints"));
}

#[tokio::test]
async fn test_phase204_acoustic_tdoa_gauss_newton_multilateration_accuracy() {
    let (mapper, mic_coords) = build_av16_3_microphone_mapper();

    let seq18_p1 = benchmarks_root().join("av16_3/seq18/person1.3dmouthgt");
    let samples = load_av16_3_mouth_ground_truth(&seq18_p1);

    // Selected pairwise microphone baselines (intra-array and inter-array pairs)
    let baseline_pairs = vec![
        (0, 4),   // array1 x-axis diameter
        (2, 6),   // array1 y-axis diameter
        (1, 5),   // array1 diagonal
        (3, 7),   // array1 diagonal
        (8, 12),  // array2 x-axis diameter
        (10, 14), // array2 y-axis diameter
        (9, 13),  // array2 diagonal
        (11, 15), // array2 diagonal
        (0, 8),   // inter-array baseline
        (4, 12),  // inter-array baseline
        (2, 10),  // inter-array baseline
        (6, 14),  // inter-array baseline
    ];

    let mut sum_sq_error_m = 0.0f32;
    let eval_count = 50.min(samples.len());
    let step = samples.len() / eval_count;

    let t0 = Instant::now();

    for i in 0..eval_count {
        let s = &samples[i * step];
        let gt = s.pos;

        // Generate synthetic pairwise TDoA measurements from real 3D mouth position with indoor reverberant jitter (sigma = 35us)
        let mut measurements = Vec::new();
        for (idx, &(m_a, m_b)) in baseline_pairs.iter().enumerate() {
            let da = gt.distance_to(&mic_coords[m_a]);
            let db = gt.distance_to(&mic_coords[m_b]);
            let true_dt = (da - db) / SPEED_OF_SOUND_MPS;

            // Pseudo-random deterministic noise in [-35us, +35us]
            let jitter_us = (((idx * 17 + i * 31) % 71) as f32 - 35.0) * 1e-6;
            let measured_dt = true_dt + jitter_us;

            let mic_a_id = format!("array{}_mic{}", if m_a < 8 { 1 } else { 2 }, (m_a % 8) + 1);
            let mic_b_id = format!("array{}_mic{}", if m_b < 8 { 1 } else { 2 }, (m_b % 8) + 1);

            measurements.push(TdoaMeasurement::new(mic_a_id, mic_b_id, measured_dt, 0.95));
        }

        let loc = AcousticRaycaster::localize_source_3d(
            &mapper,
            &measurements,
            &[],
            AcousticEventKind::HeavyImpact,
            (s.timestamp_s * 1e9) as u64,
        )
        .expect("Acoustic 3D localization must succeed");

        let err = loc.estimated_source_3d.distance_to(&gt);
        sum_sq_error_m += err * err;
    }

    let elapsed = t0.elapsed();
    let rmse_m = (sum_sq_error_m / (eval_count as f32)).sqrt();
    let mean_latency_us = elapsed.as_nanos() as f64 / (eval_count as f64 * 1000.0);

    println!(
        "\nAV16.3 Gauss-Newton Acoustic Multilateration: Evaluated {} real mouth positions -> 3D RMSE: {:.3} m ({:.1} cm), Latency: {:.1} µs/solve",
        eval_count, rmse_m, rmse_m * 100.0, mean_latency_us
    );

    // Academic target from plan: RMSE <= 15 cm in nominal acoustic conditions
    assert!(
        rmse_m <= 0.15,
        "Acoustic 3D RMSE must be <= 15 cm, got {:.2} cm",
        rmse_m * 100.0
    );
    assert!(mean_latency_us < 50.0, "Kernel solve latency must be < 50 µs");
}

#[tokio::test]
async fn test_phase204_visual_occlusion_and_cross_modal_fusion_handover() {
    let (dispatcher, scene_graph) = create_test_dispatcher().await;
    let (mapper, mic_coords) = build_av16_3_microphone_mapper();

    let seq18_p1 = benchmarks_root().join("av16_3/seq18/person1.3dmouthgt");
    let samples = load_av16_3_mouth_ground_truth(&seq18_p1);

    let baseline_pairs = vec![
        (0, 4), (2, 6), (1, 5), (3, 7),
        (8, 12), (10, 14), (9, 13), (11, 15),
        (0, 8), (4, 12), (2, 10), (6, 14),
    ];

    let person_id = 101u64;
    let person_label = "speaker_alice";

    // Scenario: 100 steps from t = 14.0s to t = 22.0s
    // - Phase 1 (t = 14.0s -> 16.5s): Nominal Optical Tracking (cam_idiap_center)
    // - Phase 2 (t = 16.5s -> 19.5s): Severe Visual Occlusion (optical drops to 0, mic_array_tdoa maintains track)
    // - Phase 3 (t = 19.5s -> 22.0s): Optical Re-acquisition (fused_kalman)
    let total_frames = 100.min(samples.len());
    let mut occlusion_errors = Vec::new();

    let base_ns = 1_700_000_000_000_000_000u64;

    {
        let mut sg = scene_graph.write();

        // Register room zone
        let zone = SpatialZone::new("smart_room_idiap", "Idiap Smart Meeting Room", -4.1, -1.8, -0.84, 8.2, 3.6, 2.4);
        sg.upsert_zone(zone);

        for i in 0..total_frames {
            let s = &samples[i];
            let ts_ns = base_ns + (s.timestamp_s * 1e9) as u64;
            let gt = s.pos;

            // Compute acoustic 3D estimate via Gauss-Newton
            let mut measurements = Vec::new();
            for (idx, &(m_a, m_b)) in baseline_pairs.iter().enumerate() {
                let da = gt.distance_to(&mic_coords[m_a]);
                let db = gt.distance_to(&mic_coords[m_b]);
                let true_dt = (da - db) / SPEED_OF_SOUND_MPS;
                let jitter_us = (((idx * 13 + i * 19) % 51) as f32 - 25.0) * 1e-6;
                let mic_a_id = format!("array{}_mic{}", if m_a < 8 { 1 } else { 2 }, (m_a % 8) + 1);
                let mic_b_id = format!("array{}_mic{}", if m_b < 8 { 1 } else { 2 }, (m_b % 8) + 1);
                measurements.push(TdoaMeasurement::new(mic_a_id, mic_b_id, true_dt + jitter_us, 0.96));
            }

            let loc = AcousticRaycaster::localize_source_3d(
                &mapper,
                &measurements,
                &[],
                AcousticEventKind::HeavyImpact,
                ts_ns,
            )
            .unwrap();

            let (stream_id, tracked_pos) = if i < 30 {
                // Phase 1: Optical Nominal
                ("cam_idiap_center".to_string(), gt)
            } else if i < 70 {
                // Phase 2: Total Optical Occlusion -> 100% Acoustic TDoA Failover
                let err = loc.estimated_source_3d.distance_to(&gt);
                occlusion_errors.push(err);
                ("mic_array_tdoa".to_string(), loc.estimated_source_3d)
            } else {
                // Phase 3: Visual Re-acquisition -> Fused Kalman
                let fused_x = 0.7 * gt.x + 0.3 * loc.estimated_source_3d.x;
                let fused_y = 0.7 * gt.y + 0.3 * loc.estimated_source_3d.y;
                let fused_z = 0.7 * gt.z + 0.3 * loc.estimated_source_3d.z;
                ("fused_kalman".to_string(), Point3D::new(fused_x, fused_y, fused_z))
            };

            let speed = if i > 0 {
                let prev_gt = samples[i - 1].pos;
                let dt = (s.timestamp_s - samples[i - 1].timestamp_s).abs() as f32;
                if dt > 1e-4 { gt.distance_to(&prev_gt) / dt } else { 0.0 }
            } else {
                0.2
            };

            sg.upsert_entity(person_id, person_label, tracked_pos, ts_ns, false);
            sg.record_waypoint(
                person_id,
                TrajectoryWaypoint {
                    timestamp_ns: ts_ns,
                    stream_id,
                    position: tracked_pos,
                    velocity_mps: Some(speed),
                    motion_state: if speed < 0.1 { MotionState::Stationary } else { MotionState::Moving },
                    bounding_box: BoundingBox2D::new(0.2, 0.2, 0.4, 0.8).unwrap(),
                    zone_id: Some("smart_room_idiap".to_string()),
                    supported_by_entity: None,
                    reference_handle: format!("handle://frame/idiap/{ts_ns}"),
                },
            );
        }
    }

    // Verify occlusion error
    assert!(!occlusion_errors.is_empty());
    let mean_occlusion_err: f32 = occlusion_errors.iter().sum::<f32>() / (occlusion_errors.len() as f32);
    println!("Visual Occlusion Window: {} frames | Mean Acoustic 3D Error: {:.2} cm (Target <= 35 cm)",
        occlusion_errors.len(), mean_occlusion_err * 100.0
    );
    assert!(mean_occlusion_err <= 0.35, "Occlusion acoustic error must be <= 35 cm, got {:.2} cm", mean_occlusion_err * 100.0);

    // Call MCP Tool locate_subject_history
    let call_args = json!({
        "target_id": person_id,
        "include_waypoints": true
    });

    let res = dispatcher
        .dispatch("locate_subject_history", Some(call_args), base_ns + (total_frames as u64) * 40_000_000)
        .await
        .expect("locate_subject_history tool dispatch must succeed");

    let text = extract_text_response(&res);
    let v: Value = serde_json::from_str(&text).expect("MCP result must be valid JSON");

    assert_eq!(v["status"], "success");
    assert_eq!(v["target_id"], person_id);
    assert_eq!(v["label"], person_label);

    let provenance = &v["sensor_modality_provenance"];
    let opt_count = provenance["optical_samples"].as_u64().unwrap();
    let aco_count = provenance["acoustic_samples"].as_u64().unwrap();
    let fus_count = provenance["fused_samples"].as_u64().unwrap();

    println!("MCP locate_subject_history provenance: Optical={}, Acoustic={}, Fused={}",
        opt_count, aco_count, fus_count
    );

    assert_eq!(opt_count, 30, "Must have exactly 30 optical samples (pre-occlusion)");
    assert_eq!(aco_count, 40, "Must have exactly 40 acoustic samples (during occlusion)");
    assert_eq!(fus_count, 30, "Must have exactly 30 fused samples (post-reacquisition)");

    assert!(v["total_distance_m"].as_f64().unwrap() > 0.0);
    assert!(v["mean_velocity_mps"].as_f64().unwrap() > 0.0);
}

#[tokio::test]
async fn test_phase204_large_scale_subject_history_benchmark() {
    let (dispatcher, scene_graph) = create_test_dispatcher().await;
    let seq18_p1 = benchmarks_root().join("av16_3/seq18/person1.3dmouthgt");
    let seq18_p2 = benchmarks_root().join("av16_3/seq18/person2.3dmouthgt");

    let s1 = load_av16_3_mouth_ground_truth(&seq18_p1);
    let s2 = load_av16_3_mouth_ground_truth(&seq18_p2);

    let base_ns = 1_700_000_000_000_000_000u64;

    // Populate scene graph with 2 speakers over 200 timestamps
    {
        let mut sg = scene_graph.write();
        let limit = 200.min(s1.len()).min(s2.len());
        for i in 0..limit {
            let ts = base_ns + (i as u64) * 40_000_000;
            sg.upsert_entity(1, "speaker_1", s1[i].pos, ts, false);
            sg.record_waypoint(1, TrajectoryWaypoint {
                timestamp_ns: ts,
                stream_id: if i % 3 == 0 { "mic_array_tdoa".to_string() } else { "cam_hall".to_string() },
                position: s1[i].pos,
                velocity_mps: Some(0.4),
                motion_state: MotionState::Moving,
                bounding_box: BoundingBox2D::new(0.1, 0.1, 0.3, 0.8).unwrap(),
                zone_id: Some("smart_room_idiap".to_string()),
                supported_by_entity: None,
                reference_handle: format!("handle://frame/{ts}"),
            });

            sg.upsert_entity(2, "speaker_2", s2[i].pos, ts, false);
            sg.record_waypoint(2, TrajectoryWaypoint {
                timestamp_ns: ts,
                stream_id: if i % 2 == 0 { "cam_hall".to_string() } else { "fused_kalman".to_string() },
                position: s2[i].pos,
                velocity_mps: Some(0.3),
                motion_state: MotionState::Moving,
                bounding_box: BoundingBox2D::new(0.5, 0.1, 0.7, 0.8).unwrap(),
                zone_id: Some("smart_room_idiap".to_string()),
                supported_by_entity: None,
                reference_handle: format!("handle://frame/{ts}"),
            });
        }
    }

    println!("\n================================================================================");
    println!(" PHASE 204: LARGE-SCALE MCP LOCATE_SUBJECT_HISTORY BENCHMARK (AV16.3 Smart Room)");
    println!(" Total Evaluated Rapid MCP Invocations: 50");
    println!("================================================================================\n");

    let mut total_latency_us = 0.0;
    let mut peak_latency_us = 0.0f64;

    for i in 0..50 {
        let target_id = if i % 2 == 0 { 1 } else { 2 };
        let call_args = json!({
            "target_id": target_id,
            "include_waypoints": true
        });

        let t0 = Instant::now();
        let res = dispatcher
            .dispatch("locate_subject_history", Some(call_args), base_ns + 10_000_000_000)
            .await
            .expect("dispatch must succeed");
        let elapsed_us = t0.elapsed().as_nanos() as f64 / 1000.0;

        total_latency_us += elapsed_us;
        if elapsed_us > peak_latency_us {
            peak_latency_us = elapsed_us;
        }

        let text = extract_text_response(&res);
        let v: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["status"], "success");
        assert_eq!(v["target_id"], target_id);
    }

    let mean_latency_us = total_latency_us / 50.0;
    println!("--------------------------------------------------------------------------------");
    println!(" MCP Performance Results (50 Invocations):");
    println!("   Mean MCP Dispatch Latency : {:.2} µs (Target < 600 µs with full waypoint serialization)", mean_latency_us);
    println!("   Peak MCP Dispatch Latency : {:.2} µs (Target < 1200 µs)", peak_latency_us);
    println!("================================================================================\n");

    assert!(mean_latency_us < 600.0, "Mean latency must be < 600 µs, got {:.2} µs", mean_latency_us);
    assert!(peak_latency_us < 1200.0, "Peak latency must be < 1200 µs, got {:.2} µs", peak_latency_us);
}
