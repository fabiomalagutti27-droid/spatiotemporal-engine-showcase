# 🌐 Spatiotemporal Semantic MCP Engine — Architecture Showcase & Benchmarks
### Edge-Native 3D Multimodal Spatial Intelligence, Zero-Copy Sensor Fusion & Sub-Millisecond Model Context Protocol

<div align="center">

[![Air-Gap Security Guard](https://img.shields.io/badge/Air--Gap%20Security-Certified%20NIST%20SP%20800--218-success.svg)](#)
[![Language](https://img.shields.io/badge/Safe_Systems_Core-Pure_Rust_2024-orange.svg)](#)
[![Performance](https://img.shields.io/badge/Throughput->=_480_FPS_(Peak_4235_FPS)-brightgreen.svg)](#)
[![Memory](https://img.shields.io/badge/Resident_RAM-22.4_MiB_(Zero--Leak_OLS)-blue.svg)](#)
[![Query Latency](https://img.shields.io/badge/MCP_Dispatch-<3.0_ms_(28_µs_probe)-purple.svg)](#)
[![Compliance](https://img.shields.io/badge/Aerospace_Standard-ECSS--E--ST--40C-red.svg)](#)
[![Quality Assurance](https://img.shields.io/badge/Product_Assurance-ECSS--Q--ST--80C_Cat_A%2FB-darkblue.svg)](#)

<br/>

**Lead Systems Architect:** **Fabio Malagutti** ([fabiomalagutti27@gmail.com](mailto:fabiomalagutti27@gmail.com))  
**Systems QA & Security Co-Architect:** **Alessandro** (`@MiServeRDC`)  
**Domain Focus:** Mission-Critical Aerospace, Autonomous Robotics & Spatial Digital Twins

<br/>

[Overview](#1-executive-overview) • [3D Interactive Telemetry](#2-interactive-3d-rerun-telemetry) • [Empirical Benchmarks](#3-certified-multi-hardware-empirical-benchmarks) • [Documentation & Whitepaper](#4-technical-whitepaper--documentation) • [Intellectual Property](#5-intellectual-property--evaluation-protocol)

<br/>

<img src="assets/media/showcase_45s.gif" alt="Spatiotemporal Semantic MCP Engine 45s Demonstration" width="100%" />

</div>

---

## 1. Executive Overview

The **Spatiotemporal Semantic MCP Engine** is a high-performance, real-time autonomous spatial intelligence engine engineered in pure, memory-safe Rust (2024 Edition). Designed as an industrial and aerospace-grade foundation for autonomous systems and multi-sensor space monitoring, it eliminates the sensory bottleneck of LLMs/VLMs by converting continuous multimodal telemetry (multi-camera optical video, Wi-Fi 6 CSI radio frequency tensors, directional acoustic arrays) into an immutable, metric, time-travelable **3D Dynamic Scene Graph**:

1. **Analytical Projective Geometry:** Formulates analytical $O(1)$ dual-homographies ($\mathbf{H}_{\text{ground}}, \mathbf{H}_{\text{head}}$) and calibrated pinhole projections $[\mathbf{K} \mid \mathbf{R}, \mathbf{t}]$ to project 2D detections into unified 3D metric coordinates, resolving severe mutual occlusions without deep monocular depth networks.
2. **Contactless Non-Visual Vital Sensing:** Processes 54-subcarrier OFDM Channel State Information (CSI) from Wi-Fi 6 nodes via Hampel filtering and Doppler STFT, tracking respiratory (16.2 RPM) and cardiac vitals (71.8 BPM) through walls and darkness.
3. **Sub-Millisecond Model Context Protocol (MCP):** Exposes 25 Context-Mode JSON-RPC 2.0 tools to AI agents (Claude, Antigravity, Cursor) with a 96% token reduction ($142\text{ tokens}$ vs $4200\text{ tokens}$) and sub-millisecond execution latencies.
4. **Mission-Critical Software Assurance:** Written with **zero `.unwrap()` / zero `.expect()`** in production code paths, strictly satisfying **ECSS-Q-ST-80C Category A/B** and **ECSS-E-ST-40C** mission-critical aerospace space software engineering standards.

---

## 2. Interactive 3D Rerun Telemetry

The repository includes pre-recorded high-fidelity 3D spatial recordings (`.rrd`) that can be replayed and inspected in the open-source **Rerun 3D Viewer**:

### A. 45-Second Master Multimodal Sequence (`showcase_45s.rrd` - 540 KB)
- **Segment 1 (00:00–00:08):** Calibrated multi-camera pinhole frustums & CAD architectural digital twin layout.
- **Segment 2 (00:08–00:22):** Multi-camera 3D tracking & continuous cross-camera Re-ID handover between rooms.
- **Segment 3 (00:22–00:34):** Contactless Wi-Fi 6 CSI vitals waveforms (16.2 RPM respiration, 71.8 BPM cardiac pulse).
- **Segment 4 (00:34–00:45):** Live agentic MCP semantic spatial queries executed in $< 3.0\text{ ms}$ with sub-200 token responses.

```bash
# Install rerun viewer (if needed)
cargo install rerun-cli --version 0.38.1 # or: pip install rerun-sdk==0.38.1

# Replay 45-second Master Demonstration
rerun assets/recordings/showcase_45s.rrd
```

### B. Genuine Multi-Camera Smart Living & Grounded Video-QA (`smart_living_demo.rrd` - 11 MB)
- **CITIC-MTMC 5-Camera Benchmark:** Continuous pedestrian cross-camera handover across 4 office rooms with real synchronized H.264 camera video streams.
- **URFD Clinical Fall Benchmark:** High-consequence fall impact detection and 12-second immobility alarm verification with 0% ADL false alarms.
- **Propose-then-Verify Grounded Video-QA:** Sub-5ms spatial-temporal question answering grounded to exact camera frames.

```bash
# Replay CITIC / URFD Genuine Multi-Camera Sequence
rerun assets/recordings/smart_living_demo.rrd
```

---

## 3. Certified Multi-Hardware Empirical Benchmarks

The engine was evaluated across 4 heterogeneous hardware tiers under identical multi-stream workloads:

| Platform & Hardware Tier | Concurrent Streams | Sustained Throughput | Peak Throughput | Resident RAM (Mach VM) | Memory Drift (OLS) | MCP Latency |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| **Apple M5 Pro (64 GB UMA)** | **32 Streams** | **806.5 FPS** | **4,235.7 FPS** | **22.4 MiB** | **0.0000 MB/hr** | **0.028 ms** |
| **Apple M5 (16 GB UMA)** | 16 Streams | 512.4 FPS | 2,180.2 FPS | 22.8 MiB | 0.0000 MB/hr | 0.034 ms |
| **Apple M1 (16 GB UMA)** | 8 Streams | 480.0 FPS | 1,420.5 FPS | 24.1 MiB | 0.0001 MB/hr | 0.048 ms |
| **NVIDIA Cloud L4 (24 GB)** | 32 Streams | 740.2 FPS | 3,890.0 FPS | 38.6 MiB | 0.0000 MB/hr | 0.031 ms |

*Detailed benchmark methodology, plots, and OLS zero-leak regression available in [`docs/BENCHMARK_REPORT.md`](docs/BENCHMARK_REPORT.md).*

---

## 4. Technical Whitepaper & Documentation

- 📄 **[Technical Architecture Whitepaper](docs/WHITEPAPER.md):** Deep mathematical formulation of dual-homographies, spatial graph algebra, and signal temporal logic.
- 📊 **[Multi-Hardware Benchmark Matrix](docs/BENCHMARK_REPORT.md):** Complete hardware comparisons, memory profiling, and throughput measurements.
- 📜 **[Dual-Authorship & IP Statement](docs/DUAL_AUTHORSHIP_STATEMENT.md):** Formal attribution statement between Fabio Malagutti (Lead Architect) and Alessandro (QA & Security).
- 🏷️ **[Product & Feature Catalog](docs/PRODUCT_FEATURE_CATALOG.md):** Industrial application catalog across 10 strategic domains.
- ⚙️ **[MCP Context-Mode JSON Schema](schemas/mcp_context_mode_schema.json):** Formal JSON-Schema definition for the 25 Model Context Protocol macro-tools.
- 🎬 **[Showcase Recording Guide](docs/SHOWCASE_DEMO_RECORDING_GUIDE.md):** Reproduction protocol for synthetic and authentic multi-camera recordings.

---

## 5. Intellectual Property & Evaluation Protocol

1. **Proprietary Monorepo Status:**  
   The underlying 16-crate Safe Rust systems codebase (~150,000 lines of code) is proprietary commercial intellectual property. All commercial rights are reserved by the authors.
2. **Institutional & Technical Review Protocol:**  
   Both authors grant explicit permission for this technical showcase, its empirical telemetry, and architectural artifacts to be evaluated by academic selection committees, industry technical boards, and aerospace research evaluators. Live interactive demonstrations and private technical code inspections are available upon formal request under confidential review conditions.
3. **Air-Gap Security Compliance:**  
   This repository is automatically validated by continuous integration workflows enforcing strict **NIST SP 800-218** isolation invariants (zero private source code leaks, zero developer paths, zero unencrypted credentials).

---

<div align="center">
  <b>Lead Systems Architect:</b> Fabio Malagutti • <a href="mailto:fabiomalagutti27@gmail.com">fabiomalagutti27@gmail.com</a><br/>
  <b>Systems QA & Security Co-Architect:</b> Alessandro (<code>@MiServeRDC</code>)
</div>
