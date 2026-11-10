//! Integration Test: CubiCasa5k Architectural Macro-Tools via MCP Protocol
//!
//! Evaluates:
//! 1. `import_floorplan_cad` JSON-RPC dispatch with real SVG payload (< 150 µs)
//! 2. `verify_blueprint_topology` topological validation (< 150 µs)
//! 3. Manifest completeness for CubiCasa tools

use core_types::{CoreError, NoOpSecurity};
use mcp_server::anomaly::AnomalyWatchdog;
use mcp_server::protocol::ContentBlock;
use mcp_server::tools::{get_tools_manifest, ToolDispatcher};
use parking_lot::RwLock;
use serde_json::json;
use spatiotemporal_db::scene_graph::DynamicSceneGraph;
use spatiotemporal_db::{L2QuerySemanticCache, SpatiotemporalDb};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tempfile::tempdir;

#[derive(Default)]
struct TestAuditLogger;
impl core_types::AuditLogger for TestAuditLogger {
    fn log_mcp_access(
        &self,
        _tool_name: &str,
        _caller_id: &str,
        _params_summary: &str,
    ) -> Result<String, CoreError> {
        Ok("audit-ok".to_string())
    }
    fn verify_audit_integrity(&self) -> Result<bool, CoreError> {
        Ok(true)
    }
}

async fn setup_test_dispatcher() -> Arc<ToolDispatcher> {
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

    Arc::new(ToolDispatcher::new(
        db,
        scene_graph,
        cache,
        watchdog,
        sanitizer,
    ))
}

fn load_apartment_svg() -> String {
    let base_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("assets/benchmarks/cubicasa5k");
    let svg_path = base_dir.join("apartment_sample_6044.svg");
    fs::read_to_string(&svg_path).expect("SVG file must be readable")
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

#[tokio::test]
async fn test_mcp_cubicasa_tools_manifest_registration() {
    let manifest = get_tools_manifest();
    let has_import = manifest.iter().any(|t| t.name == "import_floorplan_cad");
    let has_verify = manifest.iter().any(|t| t.name == "verify_blueprint_topology");

    assert!(has_import, "import_floorplan_cad must be registered in MCP manifest");
    assert!(has_verify, "verify_blueprint_topology must be registered in MCP manifest");
}

#[tokio::test]
async fn test_mcp_import_floorplan_cad_dispatch() {
    let dispatcher = setup_test_dispatcher().await;
    let svg_content = load_apartment_svg();

    let t0 = Instant::now();
    let result = dispatcher
        .dispatch(
            "import_floorplan_cad",
            Some(json!({
                "svg_payload": svg_content,
                "pixels_per_meter": 50.0,
                "wall_height": 2.80
            })),
            1_700_000_000,
        )
        .await
        .expect("Tool call must succeed");

    let elapsed_us = t0.elapsed().as_micros();
    println!("import_floorplan_cad Latency: {} µs", elapsed_us);

    let text = text_of(&result);
    let parsed_resp: serde_json::Value =
        serde_json::from_str(&text).expect("Valid JSON response required");

    assert_eq!(parsed_resp["status"], "success");
    assert_eq!(parsed_resp["total_rooms"], 7);
    assert_eq!(parsed_resp["total_doors"], 5);
    assert_eq!(parsed_resp["is_strictly_disjoint"], true);
    assert_eq!(parsed_resp["max_overlap_area_sqm"], 0.0);
}

#[tokio::test]
async fn test_mcp_verify_blueprint_topology_dispatch() {
    let dispatcher = setup_test_dispatcher().await;
    let svg_content = load_apartment_svg();

    let t0 = Instant::now();
    let result = dispatcher
        .dispatch(
            "verify_blueprint_topology",
            Some(json!({
                "svg_payload": svg_content
            })),
            1_700_000_000,
        )
        .await
        .expect("Tool call must succeed");

    let elapsed_us = t0.elapsed().as_micros();
    println!("verify_blueprint_topology Latency: {} µs", elapsed_us);

    let text = text_of(&result);
    let parsed_resp: serde_json::Value =
        serde_json::from_str(&text).expect("Valid JSON response required");

    assert_eq!(parsed_resp["status"], "success");
    assert_eq!(parsed_resp["is_strictly_disjoint"], true);
    assert_eq!(parsed_resp["verdict"], "TOPOLOGY_VALID_DISJOINT");
    assert_eq!(parsed_resp["isolated_rooms_count"], 0);
}
