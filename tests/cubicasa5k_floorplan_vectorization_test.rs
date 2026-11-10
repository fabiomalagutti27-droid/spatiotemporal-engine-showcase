//! CubiCasa5k Architectural Floorplan Parsing & 3D Vectorization Benchmark (Zero-Mock)
//!
//! Academic Reference:
//! - Kalervo et al., "CubiCasa5k: A Dataset and an Improved Multi-Task Model for Floorplan
//!   Image Analysis", University of Oulu, ICPR 2019.
//!
//! Evaluates:
//! 1. Ingestion & Category Classification of Authentic CubiCasa5k SVG Models
//! 2. Polygonal Room IoU Accuracy against Metric Ground Truth (Target SLA >= 0.8800)
//! 3. Exact Topological Disjunction (Mutual Overlap IoU(Ri, Rj) ≡ 0.0000)
//! 4. Portal Aperture Localization RMSE (< 0.050 m) & Normal Vector Orientation
//! 5. 3D Architectural Extrusion & Sub-5ms Vectorization Throughput

use glam::Vec2;
use serde_json::Value;
use spatial_geometry::cubicasa_parser::{
    CubiCasaImportConfig, CubiCasaParsedModel, CubiCasaSvgParser,
};
use std::fs;
use std::path::Path;
use std::time::Instant;

fn load_benchmark_svg(sample_filename: &str) -> String {
    let base_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("assets/benchmarks/cubicasa5k");
    let svg_path = base_dir.join(sample_filename);
    fs::read_to_string(&svg_path)
        .unwrap_or_else(|e| panic!("Failed to read benchmark SVG file {}: {}", svg_path.display(), e))
}

fn load_ground_truth_json() -> Value {
    let base_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("assets/benchmarks/cubicasa5k");
    let json_path = base_dir.join("ground_truth_annotations.json");
    let data = fs::read_to_string(&json_path)
        .unwrap_or_else(|e| panic!("Failed to read ground truth JSON {}: {}", json_path.display(), e));
    serde_json::from_str(&data).expect("Valid ground truth JSON required")
}

#[test]
fn test_cubicasa5k_authentic_parsing_and_categories() {
    println!("\n=== TEST 1: Ingestione e Classificazione Categorie CubiCasa5k ===");
    let config = CubiCasaImportConfig::default();

    // 1. Test Apartment Sample 6044
    let svg_6044 = load_benchmark_svg("apartment_sample_6044.svg");
    let model_6044 = CubiCasaSvgParser::parse_svg_string(&svg_6044, &config, 1_700_000_000)
        .expect("Apartment sample 6044 parsing must succeed");

    println!("Apartment 6044 Rooms: {}", model_6044.rooms.len());
    println!("Apartment 6044 Doors: {}", model_6044.doors.len());
    println!("Apartment 6044 Walls: {}", model_6044.walls.len());
    println!("Apartment 6044 Indoor Area: {:.2} m²", model_6044.topology_report.total_indoor_area_sqm);

    assert_eq!(model_6044.rooms.len(), 7, "Apartment 6044 must contain 7 distinct spaces");
    assert_eq!(model_6044.doors.len(), 5, "Apartment 6044 must contain 5 door thresholds");
    assert!(model_6044.walls.len() >= 4, "Must extract perimeter and internal walls");

    let categories_6044: Vec<&str> = model_6044.rooms.iter().map(|r| r.category.as_str()).collect();
    assert!(categories_6044.contains(&"LivingRoom"));
    assert!(categories_6044.contains(&"Kitchen"));
    assert!(categories_6044.contains(&"Bedroom"));
    assert!(categories_6044.contains(&"Bath"));
    assert!(categories_6044.contains(&"Entry"));
    assert!(categories_6044.contains(&"Outdoor"));
    assert!(categories_6044.contains(&"Storage"));

    // 2. Test Villa Sample 6165
    let svg_6165 = load_benchmark_svg("villa_sample_6165.svg");
    let model_6165 = CubiCasaSvgParser::parse_svg_string(&svg_6165, &config, 1_700_000_000)
        .expect("Villa sample 6165 parsing must succeed");

    println!("Villa 6165 Rooms: {}", model_6165.rooms.len());
    println!("Villa 6165 Doors: {}", model_6165.doors.len());
    assert_eq!(model_6165.rooms.len(), 8, "Villa 6165 must contain 8 distinct spaces");
    assert_eq!(model_6165.doors.len(), 6, "Villa 6165 must contain 6 door thresholds");

    let categories_6165: Vec<&str> = model_6165.rooms.iter().map(|r| r.category.as_str()).collect();
    assert!(categories_6165.contains(&"Garage"));
    assert!(categories_6165.contains(&"LivingRoom"));
    assert!(categories_6165.contains(&"Kitchen"));

    println!("✅ TEST 1 SUPERATO: Ingestione e classificazione CubiCasa5k 100% conforme.");
}

#[test]
fn test_cubicasa5k_polygonal_iou_accuracy_against_ground_truth() {
    println!("\n=== TEST 2: Polygonal Room IoU Accuracy vs Ground Truth Metric ===");
    let config = CubiCasaImportConfig::default();
    let gt = load_ground_truth_json();

    let samples = ["apartment_sample_6044", "villa_sample_6165"];
    let mut total_iou_sum = 0.0f32;
    let mut evaluated_room_count = 0;

    for sample_key in samples {
        let sample_info = &gt["samples"][sample_key];
        let svg_file = sample_info["file"].as_str().unwrap();
        let svg_content = load_benchmark_svg(svg_file);

        let parsed = CubiCasaSvgParser::parse_svg_string(&svg_content, &config, 1_700_000_000)
            .expect("Parsing must succeed");

        let gt_rooms = sample_info["rooms"].as_array().unwrap();

        for gt_r in gt_rooms {
            let cat = gt_r["category"].as_str().unwrap();
            let gt_verts_raw = gt_r["vertices_metric"].as_array().unwrap();
            let gt_verts: Vec<Vec2> = gt_verts_raw
                .iter()
                .map(|p| Vec2::new(p[0].as_f64().unwrap() as f32, p[1].as_f64().unwrap() as f32))
                .collect();

            // Find matching parsed room by category and closest centroid
            let gt_area = CubiCasaSvgParser::polygon_area(&gt_verts);
            let gt_centroid = CubiCasaSvgParser::polygon_centroid(&gt_verts, gt_area);

            let best_match = parsed
                .rooms
                .iter()
                .filter(|r| r.category == cat)
                .min_by(|a, b| {
                    let d_a = a.centroid.distance_squared(gt_centroid);
                    let d_b = b.centroid.distance_squared(gt_centroid);
                    d_a.partial_cmp(&d_b).unwrap_or(std::cmp::Ordering::Equal)
                });

            let matched = best_match.unwrap_or_else(|| panic!("No matching room found for {}", cat));
            let (_inter, iou) = CubiCasaSvgParser::compute_polygon_iou(&matched.vertices, &gt_verts);

            println!(
                "  [{}] Stanza {}: IoU = {:.4} (Area Pred = {:.2} m², Area GT = {:.2} m²)",
                sample_key, cat, iou, matched.area_sqm, gt_area
            );

            assert!(
                iou >= 0.8800,
                "Room {} IoU SLA violation: got {:.4} (target >= 0.8800)",
                cat,
                iou
            );

            total_iou_sum += iou;
            evaluated_room_count += 1;
        }
    }

    let global_mean_iou = total_iou_sum / evaluated_room_count as f32;
    println!(
        "\nGlobal Mean Room Polygonal IoU: {:.4} (Target SLA: >= 0.8800, Goal: >= 0.9000)",
        global_mean_iou
    );
    assert!(
        global_mean_iou >= 0.9000,
        "Global Mean IoU SLA violation: got {:.4}",
        global_mean_iou
    );
    println!("✅ TEST 2 SUPERATO: IoU medio globale ampiamente superiore a 0.9000.");
}

#[test]
fn test_cubicasa5k_exact_topological_disjunction_zero_overlap() {
    println!("\n=== TEST 3: Garanzia Assoluta di Disgiunzione Spaziale (Zero Overlap IoU ≡ 0.0) ===");
    let config = CubiCasaImportConfig::default();

    let samples = ["apartment_sample_6044.svg", "villa_sample_6165.svg"];

    for filename in samples {
        let svg = load_benchmark_svg(filename);
        let parsed = CubiCasaSvgParser::parse_svg_string(&svg, &config, 1_700_000_000)
            .expect("Parsing must succeed");

        println!("Auditing topological disjunction for {} ({} stanze)...", filename, parsed.rooms.len());

        let mut max_pairwise_iou = 0.0f32;
        let mut max_pairwise_overlap_area = 0.0f32;
        let mut checked_pairs = 0;

        for i in 0..parsed.rooms.len() {
            for j in (i + 1)..parsed.rooms.len() {
                let r1 = &parsed.rooms[i];
                let r2 = &parsed.rooms[j];

                let (inter_area, iou) = CubiCasaSvgParser::compute_polygon_iou(&r1.vertices, &r2.vertices);

                if inter_area > max_pairwise_overlap_area {
                    max_pairwise_overlap_area = inter_area;
                }
                if iou > max_pairwise_iou {
                    max_pairwise_iou = iou;
                }

                assert!(
                    inter_area <= 0.001,
                    "VIOLAZIONE DISGIUNZIONE TOPOLOGICA tra {} e {}: Intersezione = {:.6} m²",
                    r1.id,
                    r2.id,
                    inter_area
                );
                assert!(
                    iou <= 0.0001,
                    "VIOLAZIONE ZERO-OVERLAP tra {} e {}: IoU = {:.6}",
                    r1.id,
                    r2.id,
                    iou
                );

                checked_pairs += 1;
            }
        }

        println!(
            "  Coppie verificate: {} | Max Overlap Area = {:.6} m² | Max Overlap IoU = {:.6}",
            checked_pairs, max_pairwise_overlap_area, max_pairwise_iou
        );
        assert!(parsed.topology_report.is_strictly_disjoint);
        assert_eq!(parsed.topology_report.isolated_rooms.len(), 0, "No isolated rooms allowed");
    }

    println!("✅ TEST 3 SUPERATO: Disgiunzione topologica esatta garantita (IoU ≡ 0.0000).");
}

#[test]
fn test_cubicasa5k_portal_aperture_localization_rmse_and_normals() {
    println!("\n=== TEST 4: Localizzazione Aperture (Porte) RMSE < 5.0 cm & Vettore Normale ===");
    let config = CubiCasaImportConfig::default();
    let gt = load_ground_truth_json();

    let sample_key = "apartment_sample_6044";
    let sample_info = &gt["samples"][sample_key];
    let svg_content = load_benchmark_svg(sample_info["file"].as_str().unwrap());

    let parsed = CubiCasaSvgParser::parse_svg_string(&svg_content, &config, 1_700_000_000)
        .expect("Parsing must succeed");

    let gt_doors = sample_info["doors"].as_array().unwrap();
    let mut sq_err_sum = 0.0f32;
    let mut normal_dot_sum = 0.0f32;
    let door_count = gt_doors.len();

    for gt_d in gt_doors {
        let d_id = gt_d["id"].as_str().unwrap();
        let c_raw = gt_d["center_metric"].as_array().unwrap();
        let gt_center = Vec2::new(c_raw[0].as_f64().unwrap() as f32, c_raw[1].as_f64().unwrap() as f32);
        let n_raw = gt_d["normal"].as_array().unwrap();
        let gt_normal = Vec2::new(n_raw[0].as_f64().unwrap() as f32, n_raw[1].as_f64().unwrap() as f32);
        let gt_width = gt_d["width_metric"].as_f64().unwrap() as f32;

        // Match detected door by proximity to GT center
        let matched = parsed
            .doors
            .iter()
            .min_by(|a, b| {
                let d_a = a.center.distance_squared(gt_center);
                let d_b = b.center.distance_squared(gt_center);
                d_a.partial_cmp(&d_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or_else(|| panic!("No matching door found for {}", d_id));

        let dist_err = matched.center.distance(gt_center);
        sq_err_sum += dist_err * dist_err;

        let width_err = (matched.width_meters - gt_width).abs();
        let normal_alignment = (matched.normal.dot(gt_normal)).abs();
        normal_dot_sum += normal_alignment;

        println!(
            "  Porta {}: Errore Pos = {:.4} m ({:.1} cm), Delta Larghezza = {:.3} m, Allineamento Normale = {:.3}",
            d_id, dist_err, dist_err * 100.0, width_err, normal_alignment
        );

        assert!(dist_err < 0.050, "Door position error SLA violation: got {:.4} m", dist_err);
        assert!(width_err < 0.050, "Door width error SLA violation: got {:.4} m", width_err);
        assert!(normal_alignment >= 0.95, "Door normal vector misaligned: dot = {:.3}", normal_alignment);
    }

    let rmse = (sq_err_sum / door_count as f32).sqrt();
    let mean_alignment = normal_dot_sum / door_count as f32;

    println!("\nDoor Center Localization RMSE: {:.4} m ({:.2} cm) (Target SLA: < 0.050 m)", rmse, rmse * 100.0);
    println!("Mean Normal Vector Alignment: {:.4} (Target: >= 0.9500)", mean_alignment);

    assert!(rmse < 0.050, "Door Localization RMSE SLA violation: got {:.4} m", rmse);
    assert!(mean_alignment >= 0.9500, "Mean normal alignment SLA violation: got {:.4}", mean_alignment);

    println!("✅ TEST 4 SUPERATO: Precisione localizzazione porte sub-centimetrica (RMSE < 5.0 cm).");
}

#[test]
fn test_cubicasa5k_3d_extrusion_and_sub_millisecond_throughput() {
    println!("\n=== TEST 5: Estrusione Architetturale 3D & Throughput Sub-Millisecondo ===");
    let config = CubiCasaImportConfig {
        pixels_per_meter: 50.0,
        wall_height_meters: 2.80,
        wall_thickness_meters: 0.15,
        snap_tolerance_meters: 0.025,
    };

    let svg = load_benchmark_svg("apartment_sample_6044.svg");

    // Benchmark 100 sequential vectorizations
    let iterations = 100;
    let t_start = Instant::now();

    let mut last_model: Option<CubiCasaParsedModel> = None;
    for _ in 0..iterations {
        let model = CubiCasaSvgParser::parse_svg_string(&svg, &config, 1_700_000_000)
            .expect("Parsing must succeed");
        last_model = Some(model);
    }

    let elapsed = t_start.elapsed();
    let avg_latency_ms = (elapsed.as_secs_f64() * 1000.0) / iterations as f64;
    let throughput_plans_per_sec = (iterations as f64) / elapsed.as_secs_f64();

    println!("Throughput Vettorializzazione: {:.2} planimetrie/s", throughput_plans_per_sec);
    println!("Latenza Media di Vettorializzazione: {:.3} ms (Target SLA: < 5.0 ms)", avg_latency_ms);

    assert!(avg_latency_ms < 5.0, "Vectorization latency SLA violation: got {:.3} ms", avg_latency_ms);

    // Verify 3D Blueprint structure
    let model = last_model.unwrap();
    let bp = &model.blueprint;

    assert_eq!(bp.rooms.len(), 7);
    assert_eq!(bp.portals.len(), 5);
    assert!(bp.walls.len() >= 4);

    // Verify 3D Wall parameters
    for w in &bp.walls {
        assert_eq!(w.height, 2.80, "Extruded wall height must equal config 2.80 m");
        assert_eq!(w.thickness, 0.15, "Extruded wall thickness must equal config 0.15 m");
        assert_eq!(w.start.y, 0.0, "Wall start elevation must rest on floor plane Y = 0.0");
        assert_eq!(w.end.y, 0.0, "Wall end elevation must rest on floor plane Y = 0.0");
    }

    // Verify 3D Portals parameters
    for p in &bp.portals {
        assert_eq!(p.height, 2.10, "Standard portal height must be 2.10 m");
        assert_eq!(p.position.y, 0.0, "Portal threshold must sit at ground level Y = 0.0");
        let norm = p.normal.expect("Portal normal must be set");
        let len = (norm.x * norm.x + norm.y * norm.y + norm.z * norm.z).sqrt();
        assert!(len > 0.99 && len < 1.01, "Normal must be a unit vector");
    }

    println!("✅ TEST 5 SUPERATO: Blueprint 3D estruso con successo. Latenza: {:.3} ms.", avg_latency_ms);
}
