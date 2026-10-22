# EdgeQuake vs LightRAG — Business Performance Report

**Generated:** 2026-10-15T02:59:23Z  
**Task:** GraphRAG-Bench Acc dual-SUT (July 2026 fair pins)  
**Profile:** `ACC_E2OCC_086_v1_lrlike_arms_v2`  
**Fixture:** `medical_publish_question_ids_v1` (n=200)  
**Valid run:** `False` (eq_failed:ingest settle: entity vectors=0 < 100 for workspace 99f1c252-5ae8-4d16-beac-6f798ee4ed73 (likely saga rollback / ENOSPC))

## One-screen first principles

```text
  Task     GraphRAG-Bench/EQ-vs-LR  (same corpus · questions · judge · Mix↔Mix)
  Sample   n=200 medical-mid (50/type) — bootstrap Acc CI is underpowered at smoke n=40; this is the defendable publish ladder before full core
  Acc      EQ 0.500 · LR 0.776 · Δ -0.276
  Δ Acc CI [-0.322, -0.232] (n=200)
  L2       evidence recall EQ 0.943 / LR 0.959 · ctx_rel EQ 0.000 / LR 0.484
  Latency  query p50 EQ 0 ms / LR 1350 ms
  Verdict  LightRAG ahead on answer quality (CI excludes 0)
```

## Verdict

```text
  LightRAG ahead on answer quality (CI excludes 0)
  Acc   EdgeQuake 0.500  ·  LightRAG 0.776  ·  Δ -0.276
  Δ Acc 95% CI: [-0.322, -0.232] (n=200)
```

## What we tested

- Same GraphRAG-Bench medical questions (n=200) for both systems
- Same generator/judge stack: `mistral/mistral-small-latest` · embeddings `mistral/mistral-embed`
- Same Mix mode, matched top-k / chunk size, official `generation_eval` Acc
- Fairness: Mix arms always on, RRF fusion, chunk 1200/100, related_chunk=5
- L2 required: official `retrieval_eval` (evidence recall + context relevancy)
- **Not** UltraDomain win-rates · **not** paper Table-2 (GPT-4o-mini + BGE)
- **Why this n:** n=200 medical-mid (50/type) — bootstrap Acc CI is underpowered at smoke n=40; this is the defendable publish ladder before full core

## Scorecard for decisions

| Layer | Plain meaning | EdgeQuake | LightRAG | Winner |
|-------|---------------|-----------|----------|--------|
| Answer quality (Acc) | Are answers roughly as good? | 0.500 | 0.776 | LightRAG |
| Evidence coverage | Did we find the right sources? | 0.943 | 0.959 | LightRAG |
| Context cleanliness | Is the prompt low-noise? | 0.000 | 0.484 | LightRAG |
| Speed (query p50) | Time to answer (ms) | 0 | 1350 | EdgeQuake |

- **EQ/LR p50 ratio:** 0.0× (product SLO target ≤ 1.5×)

## By question type (Acc)

| User need | EdgeQuake | LightRAG | Who leads |
|-----------|-----------|----------|-----------|
| Fact lookup | 0.314 | 0.739 | LightRAG |
| Multi-hop reasoning | 0.579 | 0.762 | LightRAG |
| Summarization | 0.568 | 0.829 | LightRAG |
| Creative / open-ended | 0.540 | 0.774 | LightRAG |

## July 2026 landscape

On this fair Acc head-to-head, EdgeQuake is a **LightRAG-class GraphRAG peer** when Acc is statistically tied. In the GraphRAG-Bench literature (ICLR 2026), **HippoRAG2-class** systems define the aspirational **retrieval SOTA** (high evidence recall **and** high context relevancy with compact prompts). Absolute Acc numbers from the academic paper use different models and are **not** directly comparable to these Mistral Acc pins.

## Allowed / forbidden external claims

| Allowed | Forbidden |
|---------|-----------|
| “Peer / statistical tie with LightRAG on Acc under fair pins” (or “point estimate ahead” only if CI excludes 0) | “Beats LightRAG” / “wins Acc” / “#1 GraphRAG-Bench” / “SOTA RAG” without CI excluding 0 and L2 gates |
| “Peer GraphRAG with production stack (Postgres, API, PDF pipeline)” | “#1 on GraphRAG-Bench” without matching paper protocol |
| “Actively closing retrieval noise / multi-hop / latency gaps” | Silent Acc headline = CE+protect without promotion gate |
| “Publish Acc on medical-mid n=200 under fair pins” | Publishing smoke n=40 as the release score |

## How to reproduce

```bash
make bench                 # medical-mid Acc (n=200) + this publish pack
make bench-warm            # query-only (auto latest warm EQ workspace)
make bench001-smoke-acc    # daily smoke gate only (n=40; not release)
```

- Pins: Mix arms on · RRF · chunk 1200/100 · top-k 30 · `mistral/mistral-small-latest` + `mistral/mistral-embed`

## Pointers

- **This publish pack:** `specs/001-benchmark/e2e/artifacts/publish/latest/`
- **Archive:** `specs/001-benchmark/e2e/artifacts/history/medical-mid-20261015T025923Z`
- **Technical SUMMARY:** same folder / archive `SUMMARY.md`
- **Static business brief:** `specs/001-benchmark/019-business-eq-vs-lightrag-and-rag.md`
- **Acc honesty close:** `specs/001-benchmark/001-edgquake-improvements/018-e4-acc-tie-close.md`
