"""Tests for the fine-tuning pipeline — data conversion and gating logic.

These tests use synthetic data and don't require PEFT/LoRA dependencies.
"""

from __future__ import annotations

import json
import tempfile
from pathlib import Path

import pytest

from mindvault_ai.pipelines.finetune import (
    feedback_to_training_pairs,
    save_training_data,
    MIN_EVAL_SAMPLES,
)


# --- Synthetic feedback data ---

def _make_feedback(
    intent_type: str = "suggest_tag",
    action: str = "applied",
    confidence: float = 0.7,
    delta: float = 0.1,
) -> dict:
    return {
        "intent_type": intent_type,
        "action": action,
        "confidence_at_time": confidence,
        "user_edit_delta": delta,
    }


def _make_batch(n: int = 30, split: float = 0.6) -> list[dict]:
    """Generate n feedback records with given applied/dismissed split."""
    records = []
    applied_count = int(n * split)
    for i in range(n):
        action = "applied" if i < applied_count else "dismissed"
        records.append(_make_feedback(
            intent_type=["suggest_tag", "extract_task", "suggest_link"][i % 3],
            action=action,
            confidence=0.3 + (i / n) * 0.6,
        ))
    return records


# --- Tests ---


class TestFeedbackToTrainingPairs:
    def test_converts_applied_to_positive_label(self):
        pairs = feedback_to_training_pairs([_make_feedback(action="applied")])
        assert len(pairs) == 1
        assert pairs[0]["label"] == 1

    def test_converts_dismissed_to_negative_label(self):
        pairs = feedback_to_training_pairs([_make_feedback(action="dismissed")])
        assert len(pairs) == 1
        assert pairs[0]["label"] == 0

    def test_text_contains_intent_type(self):
        pairs = feedback_to_training_pairs([_make_feedback(intent_type="extract_task")])
        assert "intent:extract_task" in pairs[0]["text"]

    def test_weight_rewards_low_confidence_accepts(self):
        pair = feedback_to_training_pairs([_make_feedback(action="applied", confidence=0.2)])[0]
        # Low confidence + applied → high weight (1.5 - 0.2 = 1.3)
        assert pair["weight"] > 1.0

    def test_weight_penalizes_overconfident_rejects(self):
        pair = feedback_to_training_pairs([_make_feedback(action="dismissed", confidence=0.9)])[0]
        # High confidence + dismissed → high weight (0.9)
        assert pair["weight"] >= 0.5

    def test_batch_preserves_label_distribution(self):
        records = _make_batch(30, split=0.6)
        pairs = feedback_to_training_pairs(records)
        applied = sum(1 for p in pairs if p["label"] == 1)
        dismissed = sum(1 for p in pairs if p["label"] == 0)
        assert applied == 18  # 60% of 30
        assert dismissed == 12

    def test_handles_missing_fields_gracefully(self):
        record = {"intent_type": "suggest_link", "action": "applied"}
        pairs = feedback_to_training_pairs([record])
        assert len(pairs) == 1
        assert pairs[0]["confidence"] == 0.5  # default


class TestSaveTrainingData:
    def test_writes_jsonl_file(self):
        pairs = feedback_to_training_pairs(_make_batch(5))
        with tempfile.TemporaryDirectory() as tmp:
            path = save_training_data(pairs, Path(tmp) / "data.jsonl")
            assert path.exists()
            lines = path.read_text().strip().split("\n")
            assert len(lines) == 5
            parsed = json.loads(lines[0])
            assert "label" in parsed
            assert "text" in parsed

    def test_creates_parent_directories(self):
        pairs = feedback_to_training_pairs([_make_feedback()])
        with tempfile.TemporaryDirectory() as tmp:
            deep_path = Path(tmp) / "a" / "b" / "c" / "data.jsonl"
            result = save_training_data(pairs, deep_path)
            assert result.exists()


class TestGatingLogic:
    def test_min_eval_samples_is_20(self):
        assert MIN_EVAL_SAMPLES == 20

    def test_insufficient_data_detected(self):
        """With fewer than 20 records, the pipeline should reject."""
        records = _make_batch(15)
        pairs = feedback_to_training_pairs(records)
        assert len(pairs) == 15
        assert len(pairs) < MIN_EVAL_SAMPLES
