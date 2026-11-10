# Formal Dual-Authorship & Intellectual Property Statement

**Project:** Spatiotemporal Semantic MCP Engine  
**Release Version:** 2.4.0 (Enterprise Showcase Edition)  
**Date:** November 2026  
**Public Showcase Repository:** [github.com/fabiomalagutti27-droid/spatiotemporal-engine-showcase](https://github.com/fabiomalagutti27-droid/spatiotemporal-engine-showcase)  
**Primary Private Repository:** `spatiotemporal-mcp-engine-backup`

---

## 1. Preamble & Scope

This formal statement delineates the collaborative roles, intellectual contributions, and proprietary intellectual property (IP) framework governing the development of the **Spatiotemporal Semantic MCP Engine**. 

Developed as a private high-technology R&D initiative in memory-safe systems programming, multi-sensor spatial computing, and agentic artificial intelligence, the project represents the joint work of **Fabio Malagutti** and **Alessandro**.

---

## 2. Allocation of Architectural Responsibilities

```
+--------------------------------------------------------------------------------------------------------+
| FABIO MALAGUTTI — LEAD SYSTEMS ARCHITECT & CORE DEVELOPER                                             |
+--------------------------------------------------------------------------------------------------------+
| • Mathematical & Geometric Formulation:                                                               |
|   - Analytical derivation and implementation of Planar Dual-Homographies (H_ground, H_head).           |
|   - Pinhole camera matrix [K | R, t] projection and optical view frustum geometry.                    |
|   - Wi-Fi 6 CSI OFDM subcarrier phase unwrapping, Hampel outlier filtering, and Doppler STFT.          |
| • Systems & Core Engine Implementation:                                                               |
|   - Architecture and authoring of the 16 pure Rust crates comprising the systems runtime.             |
|   - Dynamic Spatiotemporal Scene Graph, R*-tree/Octree indexing, and LZ4-Dict WAL compactor.           |
|   - In-process Model Context Protocol (MCP) Context-Mode JSON-RPC 2.0 probe dispatcher.               |
|   - Zero-copy UMA memory pipeline (Apple VideoToolbox, NVDEC, Apache Arrow IPC).                       |
|   - Authoring of the Technical Architecture Whitepaper and Systems Engineering Dossiers.              |
+--------------------------------------------------------------------------------------------------------+
| ALESSANDRO — SYSTEMS QA, SECURITY CO-ARCHITECT & HARDWARE LEAD                                         |
+--------------------------------------------------------------------------------------------------------+
| • Systems Quality Assurance & Testbench Engineering:                                                  |
|   - Leadership of continuous integration testing, soak tests, and non-functional verification suites.   |
|   - OLS linear regression testing for memory leak verification over 100,000 continuous frames.         |
|   - Hardware testbench design: Vodafone Wi-Fi 6 Station TX illuminator + ESP32-S3 CSI sniffer.       |
| • Security Architecture & Air-Gap Enforcement:                                                        |
|   - Architecture of the dual-repository air-gap model complying with NIST SP 800-218 (SSDF).          |
|   - Static scanner rules preventing unauthorized source code (.rs) or credential leaks.               |
|   - Role-Based Access Control (RBAC), capability tokens, and deontic spatial access verification.     |
| • Hardware Benchmarking & Validation:                                                                 |
|   - Execution and certification of multi-hardware benchmarks across Apple Silicon M1, M5, M5 Pro,      |
|     and NVIDIA Cloud L4 clusters.                                                                     |
+--------------------------------------------------------------------------------------------------------+
```

---

## 3. Intellectual Property Notice & External Review Access

1. **Commercial & Proprietary Status:**  
   The underlying 16-crate Rust source code, build harnesses, and internal training scripts remain the proprietary intellectual property of the authors. All commercial rights are reserved.

2. **Public Showcase Repository Authorization:**  
   The authors have jointly authorized the public distribution of technical whitepapers, architectural Mermaid diagrams, formal JSON schemas, certified benchmark matrices, pre-recorded 3D Rerun `.rrd` telemetry sessions, and demonstration videos within the public technical showcase repository:  
   `https://github.com/fabiomalagutti27-droid/spatiotemporal-engine-showcase`.

3. **Institutional & Recruitment Review Protocol:**  
   Both authors grant explicit permission for this technical portfolio and its empirical metrics to be evaluated by academic committees, hiring bodies, and technical review boards. Live interactive demonstrations and private technical code inspections are available upon formal request under confidential review conditions.

---

**Confirmed and Endorsed,**  
**November 2026**

- **Fabio Malagutti** — Lead Systems Architect  
- **Alessandro** — Systems QA & Security Co-Architect
