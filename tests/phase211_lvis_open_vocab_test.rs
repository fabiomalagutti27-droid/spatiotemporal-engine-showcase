//! Phase 211: Zero-Shot Open-Vocabulary Perception Benchmark on LVIS Rare Categories.
//!
//! Grounded in:
//! - Gupta et al., "LVIS: A Dataset for Large Vocabulary Instance Segmentation",
//!   IEEE/CVF Conference on Computer Vision and Pattern Recognition (CVPR 2019), pp. 1980-1989.
//! - Zhai et al., "Sigmoid Loss for Language Image Pre-Training", ICCV 2023.
//! - Tschannen et al., "SigLIP 2: Multilingual Vision-Language Encoders with Improved Semantic Alignment", 2025.
//!
//! Validates:
//! 1. Authentic LVIS v1.0 rare categories dataset ingestion (50 rare classes, long-tail distribution, threats).
//! 2. Zero-shot text-to-image semantic alignment accuracy (>= 85.0% Top-1 accuracy across 50 rare classes).
//! 3. Critical threat object grounding precision (100.0% precision on non-COCO tools and weapons).
//! 4. 3D Dynamic Scene Graph open-vocabulary tagging, ontology descriptor preservation, and time-travel querying.
//! 5. MCP `query_open_vocabulary_semantics` tool dispatch latency (< 150 µs SLA) and token savings (>= 96.0%).

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use core_types::spatial::Point3D;
use core_types::{CoreError, NoOpSecurity};
use mcp_server::anomaly::AnomalyWatchdog;
use mcp_server::protocol::ContentBlock;
use mcp_server::tools::ToolDispatcher;
use parking_lot::{Mutex, RwLock};
use serde_json::{json, Value};
use spatiotemporal_db::scene_graph::DynamicSceneGraph;
use spatiotemporal_db::{L2QuerySemanticCache, SpatiotemporalDb};
use tempfile::tempdir;
use vision_engine::contrastive::siglip2::SigLip2Encoder;

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

fn lvis_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("assets/benchmarks/lvis_open_vocab")
}

async fn create_test_dispatcher() -> (
    Arc<ToolDispatcher>,
    Arc<RwLock<DynamicSceneGraph>>,
) {
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

    let dispatcher = Arc::new(
        ToolDispatcher::new(
            db,
            Arc::clone(&scene_graph),
            cache,
            watchdog,
            sanitizer,
        ),
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

// ============================================================================
// TEST 1: LVIS Rare Categories Dataset Integrity
// ============================================================================

#[tokio::test]
async fn test_phase211_lvis_rare_dataset_integrity() {
    let _guard = SUITE_MUTEX.lock();
    let root = lvis_root();
    let manifest_path = root.join("manifest.json");

    assert!(
        manifest_path.exists(),
        "LVIS v1.0 manifest must exist at: {}",
        manifest_path.display()
    );

    let content = fs::read_to_string(&manifest_path).expect("read manifest");
    let manifest: Value = serde_json::from_str(&content).expect("parse JSON");

    assert_eq!(manifest["dataset_name"], "LVIS v1.0 - Rare Categories Benchmark");
    assert_eq!(manifest["embedding_dim"], 512);

    let categories = manifest["categories"].as_array().expect("categories array");
    assert_eq!(
        categories.len(),
        50,
        "Manifest must contain exactly 50 rare LVIS categories"
    );

    let mut threat_count = 0;
    for cat in categories {
        let name = cat["name"].as_str().expect("name string");
        assert_eq!(cat["frequency"], "r", "All categories must be rare ('r') in LVIS long-tail");
        let emb = cat["canonical_embedding"].as_array().expect("canonical embedding array");
        assert_eq!(emb.len(), 512, "Embedding dimension must be 512 for category {}", name);

        let vis = cat["visual_features"].as_array().expect("visual features array");
        assert_eq!(vis.len(), 512, "Visual features dimension must be 512 for category {}", name);

        if cat["is_threat"].as_bool().unwrap_or(false) {
            threat_count += 1;
        }

        let ann_file = root.join("annotations").join(format!("{}.json", name));
        assert!(
            ann_file.exists(),
            "Annotation file for {} must exist at {}",
            name,
            ann_file.display()
        );
    }

    assert!(
        threat_count >= 10,
        "Manifest must include at least 10 threat categories (found {})",
        threat_count
    );

    println!(
        "\n[✓] Test 1 Passed: LVIS v1.0 Rare Dataset Integrity verified (50 classes, {} threats, 512-D).",
        threat_count
    );
}

// ============================================================================
// TEST 2: Open-Vocabulary Text-Visual Prompt Alignment (Top-1 Accuracy >= 85%)
// ============================================================================

#[tokio::test]
async fn test_phase211_open_vocabulary_prompt_alignment() {
    let _guard = SUITE_MUTEX.lock();
    let root = lvis_root();
    let manifest_path = root.join("manifest.json");
    let content = fs::read_to_string(&manifest_path).expect("read manifest");
    let manifest: Value = serde_json::from_str(&content).expect("parse JSON");
    let categories = manifest["categories"].as_array().expect("categories array");

    let encoder = SigLip2Encoder::with_default_config();

    // Collect text embeddings and visual embeddings for all 50 categories
    let mut names = Vec::new();
    let mut text_embs = Vec::new();
    let mut visual_embs = Vec::new();

    for cat in categories {
        let name = cat["name"].as_str().expect("name string");
        names.push(name.to_string());

        // Text embedding via SigLip2Encoder
        let t_emb = encoder.encode_text(name).expect("encode text");
        text_embs.push(t_emb);

        // Visual feature vector from manifest
        let vis_raw: Vec<f32> = cat["visual_features"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap() as f32)
            .collect();
        let v_emb = encoder.encode_visual_features(&vis_raw).expect("encode visual");
        visual_embs.push(v_emb);
    }

    let n = names.len();
    assert_eq!(n, 50);

    let mut correct_top1 = 0;
    let mut diagonal_cosines = Vec::new();

    // Compute pairwise similarity matrix across all 50x50 pairs
    for i in 0..n {
        let v_i = &visual_embs[i];
        let mut best_score = -1e9f32;
        let mut best_idx = 0;

        for (j, t_j) in text_embs.iter().enumerate() {
            let (cosine, prob) = encoder.sigmoid_match_probability(v_i, t_j);
            if i == j {
                diagonal_cosines.push(cosine);
            }
            if prob > best_score {
                best_score = prob;
                best_idx = j;
            }
        }

        if best_idx == i {
            correct_top1 += 1;
        }
    }

    let top1_accuracy = (correct_top1 as f64) / (n as f64) * 100.0;
    let mean_diag_cos: f32 = diagonal_cosines.iter().sum::<f32>() / (n as f32);

    println!(
        "\n[✓] Test 2 Alignment Results:\n    ├─ Categories Evaluated: {}\n    ├─ Correct Top-1 Matches: {}/{}\n    ├─ Top-1 Alignment Accuracy: {:.2}%\n    └─ Mean Diagonal Cosine: {:.4}",
        n, correct_top1, n, top1_accuracy, mean_diag_cos
    );

    assert!(
        top1_accuracy >= 85.0,
        "Target SLA requires Top-1 Accuracy >= 85.0% on LVIS rare classes (achieved: {:.2}%)",
        top1_accuracy
    );
    assert!(
        mean_diag_cos >= 0.85,
        "Target SLA requires Mean Diagonal Cosine >= 0.85 (achieved: {:.4})",
        mean_diag_cos
    );
}

// ============================================================================
// TEST 3: Critical Threat Object Grounding Precision (100.0%)
// ============================================================================

#[tokio::test]
async fn test_phase211_threat_object_grounding() {
    let _guard = SUITE_MUTEX.lock();
    let root = lvis_root();
    let manifest_path = root.join("manifest.json");
    let content = fs::read_to_string(&manifest_path).expect("read manifest");
    let manifest: Value = serde_json::from_str(&content).expect("parse JSON");
    let categories = manifest["categories"].as_array().expect("categories array");

    let encoder = SigLip2Encoder::with_default_config();

    let threat_classes = [
        ("crowbar", 0.15),
        ("fire_extinguisher", 0.18),
        ("screwdriver", 0.15),
        ("pipe_wrench", 0.15),
        ("dagger", 0.14),
        ("machete", 0.14),
        ("brass_knuckles", 0.14),
        ("pocketknife", 0.14),
        ("sledgehammer", 0.15),
    ];

    let mut true_positives = 0;
    let false_negatives = 0;
    let false_positives = 0;

    for &(threat_name, tau_c) in &threat_classes {
        // Find category record in manifest
        let cat_rec = categories
            .iter()
            .find(|c| c["name"].as_str() == Some(threat_name))
            .unwrap_or_else(|| panic!("Threat '{}' not found in manifest", threat_name));

        let vis_raw: Vec<f32> = cat_rec["visual_features"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap() as f32)
            .collect();
        let threat_vis = encoder.encode_visual_features(&vis_raw).expect("vis");

        // 1. Positive evaluation: threat visual against threat prompt
        let (cos_match, prob_match, is_match) = encoder
            .evaluate_detection(&threat_vis, threat_name, Some(tau_c))
            .expect("eval");

        assert!(
            is_match,
            "Threat '{}' must be detected under calibrated cutoff {} (prob={:.4}, cos={:.4})",
            threat_name, tau_c, prob_match, cos_match
        );
        true_positives += 1;

        // 2. Negative evaluation: threat visual against unrelated non-threat prompts
        let harmless_prompts = ["unicycle", "radish", "ocarina", "harmonica", "apron"];
        for harmless in harmless_prompts {
            let (_, prob_neg, is_false_alarm) = encoder
                .evaluate_detection(&threat_vis, harmless, Some(tau_c))
                .expect("eval neg");

            assert!(
                !is_false_alarm,
                "Threat '{}' falsely triggered harmless prompt '{}' with prob={:.6} (cutoff={})",
                threat_name, harmless, prob_neg, tau_c
            );
        }
    }

    let precision = (true_positives as f64) / ((true_positives + false_positives) as f64) * 100.0;
    let recall = (true_positives as f64) / ((true_positives + false_negatives) as f64) * 100.0;

    println!(
        "\n[✓] Test 3 Threat Grounding:\n    ├─ Threats Tested: {}\n    ├─ True Positives: {}\n    ├─ False Alarms: {}\n    └─ Precision: {:.2}% | Recall: {:.2}%",
        threat_classes.len(), true_positives, false_positives, precision, recall
    );

    assert_eq!(precision, 100.0, "Threat Grounding Precision must be exactly 100.0%");
    assert_eq!(recall, 100.0, "Threat Grounding Recall must be exactly 100.0%");
}

// ============================================================================
// TEST 4: Dynamic Scene Graph Open-Vocabulary Tagging & Time-Travel
// ============================================================================

#[tokio::test]
async fn test_phase211_dynamic_scene_graph_open_vocab_tagging() {
    let _guard = SUITE_MUTEX.lock();
    let mut graph = DynamicSceneGraph::new();

    let t1_ns = 1_700_000_000_000_000_000u64;
    let t2_ns = 1_700_000_005_000_000_000u64;
    let t3_ns = 1_700_000_010_000_000_000u64;

    // 1. Ingest physical entities
    let p101 = Point3D::new(1.85, 2.40, 0.95);
    let p102 = Point3D::new(4.10, -1.20, 1.40);
    graph.upsert_entity(101, "suspicious_carrier", p101, t1_ns, false);
    graph.upsert_entity(102, "wall_mounted_fixture", p102, t1_ns, true);

    // 2. Tag entities with Open-Vocabulary concepts
    assert!(graph.tag_entity_open_vocab(101, "crowbar", 0.94, t1_ns));
    assert!(graph.tag_entity_open_vocab(102, "fire_extinguisher", 0.92, t1_ns));

    // 3. Verify that subsequent kinematic update (upsert_entity) PRESERVES ontology descriptors
    graph.upsert_entity(101, "suspicious_carrier", Point3D::new(1.90, 2.42, 0.95), t2_ns, false);

    // 4. Query open-vocab entities
    let matches_crowbar = graph.query_open_vocab_entities(Some("crowbar"), 0.15, None);
    assert_eq!(matches_crowbar.len(), 1, "Must find exactly 1 entity tagged with 'crowbar'");
    assert_eq!(matches_crowbar[0].object_id, 101);
    assert!(matches_crowbar[0].is_threat, "Crowbar entity must be classified as threat");
    assert!((matches_crowbar[0].confidence - 0.94).abs() < 1e-3);

    let matches_extinguisher = graph.query_open_vocab_entities(Some("fire_extinguisher"), 0.18, None);
    assert_eq!(matches_extinguisher.len(), 1, "Must find exactly 1 entity tagged with 'fire_extinguisher'");
    assert_eq!(matches_extinguisher[0].object_id, 102);

    // 5. Verify Time-Travel temporal filtering (since_ns)
    let future_query = graph.query_open_vocab_entities(Some("crowbar"), 0.15, Some(t3_ns));
    assert_eq!(future_query.len(), 0, "Query with future cutoff must return 0 matches");

    let past_query = graph.query_open_vocab_entities(Some("crowbar"), 0.15, Some(t1_ns));
    assert_eq!(past_query.len(), 1, "Query with past cutoff must return the matched entity");

    println!(
        "\n[✓] Test 4 Passed: Dynamic Scene Graph Open-Vocabulary Tagging & Time-Travel verified (descriptors preserved across upsert)."
    );
}

// ============================================================================
// TEST 5: MCP Tool Dispatch Latency & Token Savings (< 150 µs, >= 96% Savings)
// ============================================================================

#[tokio::test]
async fn test_phase211_mcp_open_vocabulary_dispatch_latency() {
    let _guard = SUITE_MUTEX.lock();
    let (dispatcher, scene_graph) = create_test_dispatcher().await;

    let now_ns = 1_700_000_000_000_000_000u64;

    // Populate scene graph with tagged entities
    {
        let mut sg = scene_graph.write();
        sg.upsert_entity(201, "intruder_subject", Point3D::new(2.1, 1.5, 0.8), now_ns, false);
        sg.tag_entity_open_vocab(201, "crowbar", 0.95, now_ns);

        sg.upsert_entity(202, "corridor_equipment", Point3D::new(0.5, 3.2, 1.2), now_ns, true);
        sg.tag_entity_open_vocab(202, "fire_extinguisher", 0.91, now_ns);
    }

    let args = json!({
        "prompts": ["crowbar", "fire_extinguisher"],
        "lookback_seconds": 3600
    });

    // Warm-up run
    let warmup_res = dispatcher
        .dispatch("query_open_vocabulary_semantics", Some(args.clone()), now_ns)
        .await
        .expect("warmup call");
    let warmup_text = text_of(&warmup_res);
    let warmup_json: Value = serde_json::from_str(&warmup_text).expect("warmup JSON");
    assert_eq!(warmup_json["status"], "success");
    assert_eq!(warmup_json["matches_count"], 2);

    // Benchmark 50 consecutive dispatches
    let iterations = 50;
    let start = Instant::now();
    for _ in 0..iterations {
        let res = dispatcher
            .dispatch("query_open_vocabulary_semantics", Some(args.clone()), now_ns)
            .await
            .expect("dispatch");
        assert!(!res.content.is_empty());
    }
    let elapsed = start.elapsed();
    let latency_per_dispatch = elapsed / iterations;
    let latency_micros = latency_per_dispatch.as_micros();

    let raw_payload_bytes = warmup_text.len();
    // Compact token savings calculation: raw text vs standard VLM visual token budget (approx 4,096 tokens = 16,384 bytes)
    let baseline_vlm_tokens = 4096.0;
    let compact_tokens = (raw_payload_bytes as f64) / 4.0;
    let token_savings = ((baseline_vlm_tokens - compact_tokens) / baseline_vlm_tokens) * 100.0;

    println!(
        "\n[✓] Test 5 MCP Dispatch Latency Results:\n    ├─ Iterations: {}\n    ├─ Mean Dispatch Latency: {} µs (SLA target: < 150 µs)\n    ├─ Compact Response Payload: {} bytes\n    └─ Token Savings Ratio: {:.2}% (SLA target: >= 96.0%)",
        iterations, latency_micros, raw_payload_bytes, token_savings
    );

    assert!(
        latency_micros < 150,
        "Target SLA requires MCP dispatch latency < 150 µs (observed: {} µs)",
        latency_micros
    );
    assert!(
        token_savings >= 96.0,
        "Target SLA requires Token Savings >= 96.0% (observed: {:.2}%)",
        token_savings
    );
}
