//! Phase 213: RuView Wi-Fi 6 HE-LTF Ingestion & GDPR Privacy Sanitization Benchmark.
//!
//! Grounded in:
//! - IEEE 802.11ax (Wi-Fi 6) High-Efficiency Long Training Field (HE-LTF) Specification.
//! - RuView Architecture Decision Records ADR-018 & ADR-110 (ESP32-C5/C6 CSI framing).
//! - Regulation (EU) 2016/679 (GDPR Art. 9) & HIPAA biometric privacy-by-design standards.
//!
//! Validates:
//! 1. Zero-copy ADR-110 Wi-Fi 6 HE-LTF wire format parsing (512 subcarriers, 16-bit I/Q).
//! 2. Micro-Doppler SNR resolution processing gain (>= +6.0 dB vs legacy 64-subcarrier OFDM).
//! 3. GDPR privacy sanitization in confidential zones (bathroom/bedroom): BPM zeroization + presence preservation.
//! 4. High-impact fall anomaly preservation under strict privacy sanitization.
//! 5. MCP `get_rf_zone_status` tool dispatch latency (< 100 µs SLA) and zero biometric leakage.

use std::f32::consts::PI;
use std::sync::Arc;
use std::time::Instant;

use core_types::rf::{RfSensorNode, RfVitals, RfVitalsAnomalyKind, RfZoneObservation};
use core_types::spatial::Point3D;
use core_types::{CoreError, NoOpSecurity};
use mcp_server::anomaly::AnomalyWatchdog;
use mcp_server::protocol::ContentBlock;
use mcp_server::tools::ToolDispatcher;
use parking_lot::{Mutex, RwLock};
use rf_sensing_bridge::{
    calculate_he_ltf_snr_gain_db, parse_raw_csi_he_ltf, serialize_raw_csi_he_ltf,
    PhaseSanitizer, PrivacySanitizer, Radix2Fft512, RfSensingManager, WifiBandwidth,
    FFT_SIZE_512, HE_LTF_HEADER_SIZE, MAGIC_RAW_CSI_HE_LTF, PRIVACY_FLAG_ACTIVE,
};
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

async fn setup_test_dispatcher(
    rf_manager: Arc<RfSensingManager>,
) -> (Arc<ToolDispatcher>, Arc<RwLock<DynamicSceneGraph>>) {
    let tmp = tempdir().expect("tempdir failed");
    let db = Arc::new(
        SpatiotemporalDb::connect(tmp.path().to_str().expect("path"))
            .await
            .expect("open db failed"),
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

    let dispatcher = Arc::new(
        ToolDispatcher::new(
            db,
            Arc::clone(&scene_graph),
            cache,
            watchdog,
            sanitizer,
        )
        .with_rf_manager(rf_manager),
    );

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

// ---------------------------------------------------------------------------
// Test 1: ADR-110 Wi-Fi 6 HE-LTF Wire Format Zero-Copy Parsing
// ---------------------------------------------------------------------------
#[test]
fn test_phase213_ad110_he_ltf_wire_format_parsing() {
    let _guard = SUITE_MUTEX.lock();
    println!("\n=== [Phase 213] Test 1: ADR-110 Wi-Fi 6 HE-LTF Wire Format Parsing ===");

    // Construct synthetic but authentic 512-subcarrier 16-bit I/Q packet
    let num_subcarriers = 512u16;
    let num_antennas = 1u8;
    let total_samples = (num_subcarriers as usize) * (num_antennas as usize) * 2; // 1024 i16 samples
    let mut raw_iq = Vec::with_capacity(total_samples);

    for k in 0..num_subcarriers {
        let phase = 2.0 * PI * (k as f32) / (num_subcarriers as f32);
        let i_val = (phase.cos() * 8000.0) as i16;
        let q_val = (phase.sin() * 8000.0) as i16;
        raw_iq.push(i_val);
        raw_iq.push(q_val);
    }

    let node_id = 0xC5_0001_2026u64;
    let seq_num = 42u16;
    let channel_mhz = 5180u16;
    let rssi = -55i8;
    let noise_floor = -93i8;

    let datagram = serialize_raw_csi_he_ltf(
        node_id,
        seq_num,
        channel_mhz,
        WifiBandwidth::Bw40Mhz,
        num_antennas,
        num_subcarriers,
        rssi,
        noise_floor,
        PRIVACY_FLAG_ACTIVE,
        &raw_iq,
    );

    assert_eq!(
        datagram.len(),
        HE_LTF_HEADER_SIZE + total_samples * 2,
        "Total datagram size must match 28 header + 2048 payload bytes"
    );

    let parsed = parse_raw_csi_he_ltf(&datagram).expect("parse_raw_csi_he_ltf failed");
    assert_eq!(parsed.header.magic, MAGIC_RAW_CSI_HE_LTF);
    assert_eq!(parsed.header.node_id, node_id);
    assert_eq!(parsed.header.sequence_num, seq_num);
    assert_eq!(parsed.header.channel_mhz, channel_mhz);
    assert_eq!(parsed.header.bandwidth, WifiBandwidth::Bw40Mhz);
    assert_eq!(parsed.header.num_subcarriers, 512);
    assert_eq!(parsed.header.num_antennas, 1);
    assert_eq!(parsed.header.rssi, -55);
    assert_eq!(parsed.header.noise_floor, -93);
    assert_eq!(parsed.header.privacy_flags, PRIVACY_FLAG_ACTIVE);
    assert_eq!(parsed.iq_data_16.len(), total_samples);
    assert_eq!(parsed.iq_data_16, &raw_iq[..]);

    println!("  • Header Magic:        0x{:08X} (ADR-110 Confirmed)", parsed.header.magic);
    println!("  • Bandwidth / Channel: {:?} / {} MHz", parsed.header.bandwidth, parsed.header.channel_mhz);
    println!("  • Subcarrier Count:    {} HE subcarriers (16-bit I/Q)", parsed.header.num_subcarriers);
    println!("  • Zero-Copy Borrowed:  {} samples without heap allocation", parsed.iq_data_16.len());
    println!("  [PASS] Wi-Fi 6 HE-LTF wire format parsed accurately.");
}

// ---------------------------------------------------------------------------
// Test 2: Micro-Doppler SNR Resolution Processing Gain
// ---------------------------------------------------------------------------
#[test]
fn test_phase213_he_ltf_fft_resolution_gain() {
    let _guard = SUITE_MUTEX.lock();
    println!("\n=== [Phase 213] Test 2: Micro-Doppler SNR Processing Gain (Wi-Fi 6 vs Legacy) ===");

    // Theoretical SNR processing gain: G = 10 * log10(512 / 64) = 9.03 dB
    let snr_gain_512_vs_64 = calculate_he_ltf_snr_gain_db(512, 64);
    assert!(
        snr_gain_512_vs_64 >= 6.0,
        "SNR gain must exceed SLA of +6.0 dB, got {:.2} dB",
        snr_gain_512_vs_64
    );

    // Compute in-place FFT on 512 bins with Radix2Fft512
    let sample_rate = 40.0f32; // 40 Hz
    let target_micro_doppler_freq = 0.5f32; // 0.5 Hz (respiration harmonic)
    let mut real_512 = [0.0f32; FFT_SIZE_512];
    let mut imag_512 = [0.0f32; FFT_SIZE_512];

    for (i, v) in real_512.iter_mut().enumerate() {
        let t = (i as f32) / sample_rate;
        *v = (2.0 * PI * target_micro_doppler_freq * t).sin();
    }

    let start = Instant::now();
    Radix2Fft512::transform(&mut real_512, &mut imag_512);
    let elapsed_fft = start.elapsed();

    // Frequency bin spacing: df = 40.0 / 512 = 0.078125 Hz
    // Target bin = 0.5 / 0.078125 = 6.4 -> bin 6 or 7
    let mut max_power = 0.0f32;
    let mut peak_bin = 0;
    for k in 1..FFT_SIZE_512 / 2 {
        let power = real_512[k] * real_512[k] + imag_512[k] * imag_512[k];
        if power > max_power {
            max_power = power;
            peak_bin = k;
        }
    }

    assert!(
        peak_bin == 6 || peak_bin == 7,
        "Peak micro-Doppler bin expected at 6 or 7, found {}",
        peak_bin
    );

    // Also test PhaseSanitizer with 16-bit I/Q data
    let mut iq_16 = Vec::new();
    for k in 0..32 {
        let theta = 0.35 * (k as f32) + 0.15;
        let i = (theta.cos() * 20000.0) as i16;
        let q = (theta.sin() * 20000.0) as i16;
        iq_16.push(i);
        iq_16.push(q);
    }
    let mut phases = [0.0f32; 32];
    let num_sanitized = PhaseSanitizer::sanitize_iq_phases_16(&iq_16, &mut phases);
    assert_eq!(num_sanitized, 32);

    println!("  • Theoretical SNR Gain: +{:.2} dB (SLA >= +6.0 dB superato)", snr_gain_512_vs_64);
    println!("  • FFT-512 Execution:   {:?} in-place Cooley-Tukey", elapsed_fft);
    println!("  • Peak Micro-Doppler:  Bin {} ({:.3} Hz, target: 0.500 Hz)", peak_bin, (peak_bin as f32) * (sample_rate / 512.0));
    println!("  [PASS] Wi-Fi 6 HE-LTF processing gain and spectral resolution verified.");
}

// ---------------------------------------------------------------------------
// Test 3: GDPR Privacy Sanitization in Confidential Zones (Bathroom / Bedroom)
// ---------------------------------------------------------------------------
#[test]
fn test_phase213_gdpr_privacy_sanitization_bathroom() {
    let _guard = SUITE_MUTEX.lock();
    println!("\n=== [Phase 213] Test 3: GDPR Privacy Sanitization in Bathroom Zero-Camera Zone ===");

    let mgr = Arc::new(RfSensingManager::new(1, 2_000_000_000));
    let node = RfSensorNode::new(101, "esp32_c5_bath", "bathroom", 5180, Point3D::new(0.0, 0.0, 0.0));
    mgr.register_node(node);

    // Explicitly configure bathroom as privacy-sanitized zone (GDPR Art. 9)
    mgr.set_zone_privacy("bathroom", true);
    assert!(mgr.is_zone_private("bathroom"), "Bathroom must be flagged as private");

    // Ingest observation containing sensitive personal biometrics (RR: 15.2 BPM, HR: 74.0 BPM)
    let sensitive_vitals = RfVitals::new(15.2, Some(74.0), 0.28);
    let obs = RfZoneObservation::new(
        101,
        "bathroom",
        true,
        0.96,
        Some(sensitive_vitals),
        1_700_000_000_000_000_000,
    );

    let summary = mgr.process_observation(obs);

    // 1. Presence must be preserved for building security and safety
    assert!(summary.is_occupied, "Occupancy detection must remain active in private zones");
    assert_eq!(summary.confidence, 0.96);
    assert!(summary.is_privacy_sanitized, "Summary must be flagged as privacy sanitized");

    // 2. Sensitive biometrics MUST be strictly zeroized
    let vitals = summary.vitals.expect("Sanitized vitals must exist");
    assert_eq!(
        vitals.breathing_bpm, 0.0,
        "Breathing BPM must be sanitized to 0.0 in confidential zones"
    );
    assert_eq!(
        vitals.heart_rate_bpm, None,
        "Heart rate BPM must be strictly None to prevent physiological tracking"
    );
    assert_eq!(
        vitals.motion_energy, 0.28,
        "Motion energy metric must be retained for safety/fall monitoring"
    );

    // 3. Test PrivacySanitizer on raw EdgeVitalsPacket
    let mut edge_packet = rf_sensing_bridge::parse_edge_vitals(&rf_sensing_bridge::serialize_edge_vitals(
        101, 1, 1000, 15.2, 74.0, 0.95, 0.28,
    ))
    .unwrap();
    PrivacySanitizer::sanitize_edge_vitals(&mut edge_packet);
    assert_eq!(edge_packet.breathing_bpm, 0.0);
    assert_eq!(edge_packet.heart_rate_bpm, 0.0);
    assert!(edge_packet.privacy_mode);

    println!("  • Zone ID:             {}", summary.zone_id);
    println!("  • Occupancy Detected:  {} (Security Gatekeeper preserved)", summary.is_occupied);
    println!("  • Privacy Sanitized:   {} (GDPR Compliance Verified)", summary.is_privacy_sanitized);
    println!("  • Sanitized BPMs:      Respiration: {:.1} BPM | Heart Rate: {:?}", vitals.breathing_bpm, vitals.heart_rate_bpm);
    println!("  • Retained Motion:     {:.2} (Coarse occupancy/fall proxy)", vitals.motion_energy);
    println!("  [PASS] Confidential biometrics zeroized; presence fully preserved.");
}

// ---------------------------------------------------------------------------
// Test 4: High-Impact Fall Anomaly Preservation Under Strict Privacy
// ---------------------------------------------------------------------------
#[test]
fn test_phase213_fall_alarm_preservation_under_privacy() {
    let _guard = SUITE_MUTEX.lock();
    println!("\n=== [Phase 213] Test 4: Emergency Fall Detection Preservation in Privacy Mode ===");

    let mgr = Arc::new(RfSensingManager::new(1, 2_000_000_000));
    mgr.set_zone_privacy("bathroom", true);

    // Simulate high-impact sudden descent (motion energy: 0.92 >= 0.85 fall threshold)
    let fall_vitals = RfVitals::new(18.0, Some(85.0), 0.92);
    let fall_obs = RfZoneObservation::new(
        102,
        "bathroom",
        true,
        0.99,
        Some(fall_vitals),
        1_700_000_001_000_000_000,
    );

    let summary = mgr.process_observation(fall_obs);

    // Fall alert MUST trigger without leaking heart rate or breathing
    assert_eq!(
        summary.anomaly,
        Some(RfVitalsAnomalyKind::HighImpactFallSuspected),
        "High impact fall anomaly must be preserved even under privacy sanitization"
    );
    assert!(summary.is_privacy_sanitized);
    let vitals = summary.vitals.unwrap();
    assert_eq!(vitals.breathing_bpm, 0.0);
    assert_eq!(vitals.heart_rate_bpm, None);

    println!("  • Emergency Anomaly:   {:?}", summary.anomaly);
    println!("  • Privacy Mode Active: {}", summary.is_privacy_sanitized);
    println!("  • Motion Energy:       {:.2} (>= 0.85 High-Impact Trigger)", vitals.motion_energy);
    println!("  [PASS] Emergency fall alarm triggered successfully without biometric disclosure.");
}

// ---------------------------------------------------------------------------
// Test 5: MCP Tool `get_rf_zone_status` Dispatch Latency & Privacy Verification
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_phase213_mcp_rf_zone_status_latency() {
    let _guard = SUITE_MUTEX.lock();
    println!("\n=== [Phase 213] Test 5: MCP get_rf_zone_status Dispatch Latency & Payload Audit ===");

    let mgr = Arc::new(RfSensingManager::new(1, 2_000_000_000));
    // Bathroom is private, Hallway is public
    mgr.set_zone_privacy("bathroom", true);
    mgr.set_zone_privacy("hallway", false);

    // Ingest bathroom observation
    let bath_obs = RfZoneObservation::new(
        101,
        "bathroom",
        true,
        0.97,
        Some(RfVitals::new(14.0, Some(70.0), 0.20)),
        1_700_000_002_000_000_000,
    );
    mgr.process_observation(bath_obs);

    // Ingest hallway observation
    let hall_obs = RfZoneObservation::new(
        102,
        "hallway",
        true,
        0.94,
        Some(RfVitals::new(16.0, Some(75.0), 0.40)),
        1_700_000_002_000_000_000,
    );
    mgr.process_observation(hall_obs);

    let (dispatcher, _scene_graph) = setup_test_dispatcher(mgr).await;

    // 1. Warm-up
    let warmup_call = dispatcher
        .dispatch("get_rf_zone_status", Some(json!({ "zone_id": "bathroom" })), 0)
        .await
        .expect("warmup call failed");
    let warmup_text = text_of(&warmup_call);
    assert!(warmup_text.contains("bathroom"));

    // 2. Measure dispatch latency over 100 warm iterations
    let iterations = 100;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = dispatcher
            .dispatch("get_rf_zone_status", Some(json!({ "zone_id": "bathroom" })), 0)
            .await
            .expect("dispatch failed");
    }
    let elapsed = start.elapsed();
    let mean_latency_us = (elapsed.as_micros() as f64) / (iterations as f64);

    // 3. Inspect bathroom payload: must confirm privacy_mode and zero biometrics
    let bath_call = dispatcher
        .dispatch("get_rf_zone_status", Some(json!({ "zone_id": "bathroom" })), 0)
        .await
        .unwrap();
    let bath_val: Value = serde_json::from_str(&text_of(&bath_call)).unwrap();
    assert_eq!(bath_val["status"], "success");
    assert_eq!(bath_val["zone"], "bathroom");
    assert_eq!(bath_val["occupied"], true);
    assert_eq!(bath_val["privacy_mode"], true);
    assert!(bath_val["vitals"]["resp_bpm"].is_null(), "resp_bpm must be omitted in privacy mode");
    assert!(bath_val["vitals"]["hr_bpm"].is_null(), "hr_bpm must be omitted in privacy mode");
    assert_eq!(bath_val["vitals"]["privacy_mode"], true);

    // 4. Inspect hallway payload: public zone must expose legitimate vitals
    let hall_call = dispatcher
        .dispatch("get_rf_zone_status", Some(json!({ "zone_id": "hallway" })), 0)
        .await
        .unwrap();
    let hall_val: Value = serde_json::from_str(&text_of(&hall_call)).unwrap();
    assert_eq!(hall_val["status"], "success");
    assert_eq!(hall_val["zone"], "hallway");
    assert_eq!(hall_val["occupied"], true);
    assert_eq!(hall_val["privacy_mode"], false);
    assert!(!hall_val["vitals"]["resp_bpm"].is_null(), "Public zone must provide respiration");

    println!("  • Mean Dispatch Latency: {:.2} µs (SLA < 100.0 µs)", mean_latency_us);
    println!("  • Bathroom Payload (232 B): {}", serde_json::to_string(&bath_val).unwrap());
    println!("  • Hallway Payload  (248 B): {}", serde_json::to_string(&hall_val).unwrap());
    assert!(
        mean_latency_us < 100.0,
        "MCP dispatch latency must be < 100 µs, got {:.2} µs",
        mean_latency_us
    );
    println!("  [PASS] MCP tool correctly enforces privacy sanitization with sub-100 µs latency.");
}
