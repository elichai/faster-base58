# Fuzz Corpus Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep a validated cross-platform fuzz corpus on an orphan branch and report production coverage to Codecov.

**Architecture:** Main contains a small Python corpus tool and CI workflows. The orphan branch is data-only. Fuzz and coverage jobs replay a pinned corpus snapshot; nightly artifact collection is separate from the publishing job.

**Tech Stack:** Python standard library, cargo-fuzz/libFuzzer, GitHub Actions, LLVM LCOV export, Codecov OIDC.

**Spec:** `docs/fuzzing-corpus.md`

## Global Constraints

- Never modify the locally downloaded untracked corpora.
- Validate all artifact-originated corpus files before they are replayed or published.
- Do not merge LLVM profile data across architecture, target CPU, or feature configuration.
- Keep publication write permission isolated from pull requests and workers.

## Review Focus

- Symlink, traversal, and non-hash corpus filenames must fail validation.
- Duplicate input bytes must canonicalize to one SHA-256 file.
- A publisher must add inputs only; it must not replace a corpus revision.
- A coverage filter must omit diagnostics, tests, and fuzz harnesses.
- Codecov uploads must be distinct by source of coverage and configuration.

### Task 1: Corpus validator and staging tool

**Files:**
- Create: `fuzz/corpus_store.py`
- Create: `fuzz/test_corpus_store.py`

- [ ] Write failing tests for file validation and SHA-256 canonicalization.
- [ ] Run the tests and observe missing-module failure.
- [ ] Implement the minimal validator and additive staging command.
- [ ] Run the focused tests.

### Task 2: Corpus CI

**Files:**
- Create: `.github/workflows/fuzz-corpus.yml`
- Modify: `fuzz/Cargo.toml`
- Modify: `fuzz/README.md`

- [ ] Add a data-only corpus checkout, replay matrix, nightly artifacts, and isolated publisher.
- [ ] Validate workflow structure with the existing CI checker and Python self-tests.

### Task 3: Production coverage uploads

**Files:**
- Modify: `.github/workflows/verification.yml`
- Create: `codecov.yml`

- [ ] Add LCOV export and separate Codecov flags for fuzz coverage and property tests.
- [ ] Exclude non-production sources and validate the workflow checker.
