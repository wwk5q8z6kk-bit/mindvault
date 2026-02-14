"""Fine-tuning pipeline — LoRA via peft (optional dependencies).

Implements the AgentEvolver cloud-train PoC:
1. Export feedback records from the vault as JSONL training pairs
2. LoRA fine-tune the embedding model on applied/dismissed signals
3. Evaluate on holdout set with N≥20 gating
4. Serialize adapted weights for cloud handoff
"""

from __future__ import annotations

import json
import time
import uuid
from pathlib import Path
from typing import Any

import httpx
import structlog

logger = structlog.get_logger()

# Default vault endpoint for feedback export
VAULT_BASE_URL = "http://127.0.0.1:9470"
MIN_EVAL_SAMPLES = 20


def is_available() -> bool:
    """Check if fine-tuning dependencies are installed."""
    try:
        import peft  # noqa: F401
        import datasets  # noqa: F401
        import accelerate  # noqa: F401

        return True
    except ImportError:
        return False


# ---------- Training data export ----------


async def export_feedback(
    vault_url: str = VAULT_BASE_URL,
    limit: int = 500,
    intent_type: str | None = None,
    auth_token: str | None = None,
) -> list[dict[str, Any]]:
    """Fetch feedback records from the MindVault vault REST API.

    Returns raw AgentFeedback records as dicts.
    """
    params: dict[str, Any] = {"limit": limit}
    if intent_type:
        params["intent_type"] = intent_type

    headers: dict[str, str] = {}
    if auth_token:
        headers["Authorization"] = f"Bearer {auth_token}"

    async with httpx.AsyncClient(timeout=30) as client:
        resp = await client.get(
            f"{vault_url}/api/v1/agent/feedback",
            params=params,
            headers=headers,
        )
        resp.raise_for_status()
        return resp.json()


def feedback_to_training_pairs(
    records: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    """Convert feedback records to contrastive training pairs.

    Each pair has:
    - text: the intent type + context description
    - label: 1 for applied (positive), 0 for dismissed (negative)
    - confidence: original confidence at time of action
    - weight: importance weight (higher for low-confidence correct predictions)
    """
    pairs = []
    for rec in records:
        action = rec.get("action", "")
        intent_type = rec.get("intent_type", "unknown")
        confidence = rec.get("confidence_at_time") or 0.5
        delta = rec.get("user_edit_delta") or 0.0

        label = 1 if action == "applied" else 0

        # Weight: reward correct low-confidence predictions and penalize
        # overconfident wrong ones
        if label == 1:
            weight = max(0.5, 1.5 - confidence)  # Higher weight for surprising accepts
        else:
            weight = max(0.5, confidence)  # Higher weight for overconfident rejects

        pairs.append({
            "text": f"intent:{intent_type} confidence:{confidence:.2f} delta:{delta:.2f}",
            "label": label,
            "confidence": confidence,
            "weight": weight,
            "intent_type": intent_type,
            "source_action": action,
        })

    return pairs


def save_training_data(pairs: list[dict[str, Any]], output_path: Path) -> Path:
    """Write training pairs to JSONL file."""
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, "w") as f:
        for pair in pairs:
            f.write(json.dumps(pair) + "\n")
    logger.info("training_data_saved", path=str(output_path), count=len(pairs))
    return output_path


# ---------- LoRA fine-tuning ----------


async def start_finetune(params: dict[str, Any]) -> dict[str, Any]:
    """Start a fine-tuning job using PEFT/LoRA on feedback data.

    Params:
        model_name: base model to fine-tune (default: embedding model)
        vault_url: vault REST API URL for feedback export
        auth_token: optional auth token for vault API
        intent_type: filter feedback by intent type
        output_dir: where to save adapted model
        epochs: training epochs (default: 3)
        lr: learning rate (default: 1e-4)
        lora_r: LoRA rank (default: 8)
        lora_alpha: LoRA alpha (default: 16)
        batch_size: training batch size (default: 4)
        eval_split: fraction held out for evaluation (default: 0.2)
    """
    if not is_available():
        return {
            "status": "error",
            "message": "Fine-tuning dependencies not installed. Install with: uv pip install 'mindvault-ai[finetune]'",
        }

    job_id = str(uuid.uuid4())[:8]
    vault_url = params.get("vault_url", VAULT_BASE_URL)
    auth_token = params.get("auth_token")
    intent_type = params.get("intent_type")
    output_dir = Path(params.get("output_dir", "~/.mindvault/finetune")).expanduser()
    epochs = params.get("epochs", 3)
    lr = params.get("lr", 1e-4)
    lora_r = params.get("lora_r", 8)
    lora_alpha = params.get("lora_alpha", 16)
    batch_size = params.get("batch_size", 4)
    eval_split = params.get("eval_split", 0.2)

    logger.info("finetune_starting", job_id=job_id)

    # Step 1: Export feedback data
    try:
        raw_feedback = await export_feedback(
            vault_url=vault_url,
            limit=500,
            intent_type=intent_type,
            auth_token=auth_token,
        )
    except Exception as e:
        logger.warning("feedback_export_failed", error=str(e))
        raw_feedback = []

    if not raw_feedback:
        return {
            "status": "error",
            "job_id": job_id,
            "message": "No feedback records found. Collect feedback via /api/v1/agent/feedback first.",
        }

    # Step 2: Convert to training pairs
    pairs = feedback_to_training_pairs(raw_feedback)
    if len(pairs) < MIN_EVAL_SAMPLES:
        return {
            "status": "error",
            "job_id": job_id,
            "message": f"Need at least {MIN_EVAL_SAMPLES} samples, got {len(pairs)}. Collect more feedback.",
        }

    # Save training data
    data_path = output_dir / job_id / "training_data.jsonl"
    save_training_data(pairs, data_path)

    # Step 3: Run LoRA fine-tuning
    result = await _run_lora_training(
        pairs=pairs,
        job_id=job_id,
        output_dir=output_dir / job_id,
        epochs=epochs,
        lr=lr,
        lora_r=lora_r,
        lora_alpha=lora_alpha,
        batch_size=batch_size,
        eval_split=eval_split,
    )

    return result


async def _run_lora_training(
    pairs: list[dict[str, Any]],
    job_id: str,
    output_dir: Path,
    epochs: int,
    lr: float,
    lora_r: int,
    lora_alpha: int,
    batch_size: int,
    eval_split: float,
) -> dict[str, Any]:
    """Execute LoRA training on CPU (Intel Mac compatible)."""
    import torch
    from datasets import Dataset
    from peft import LoraConfig, TaskType, get_peft_model
    from transformers import (
        AutoModelForSequenceClassification,
        AutoTokenizer,
        Trainer,
        TrainingArguments,
    )

    t0 = time.time()
    output_dir.mkdir(parents=True, exist_ok=True)

    # Use a small model suitable for CPU training
    base_model_name = "distilbert-base-uncased"

    logger.info("loading_base_model", model=base_model_name, job_id=job_id)
    tokenizer = AutoTokenizer.from_pretrained(base_model_name)
    model = AutoModelForSequenceClassification.from_pretrained(
        base_model_name,
        num_labels=2,
        torch_dtype=torch.float32,
    )

    # Apply LoRA
    lora_config = LoraConfig(
        task_type=TaskType.SEQ_CLS,
        r=lora_r,
        lora_alpha=lora_alpha,
        lora_dropout=0.1,
        target_modules=["q_lin", "v_lin"],
    )
    model = get_peft_model(model, lora_config)
    trainable = sum(p.numel() for p in model.parameters() if p.requires_grad)
    total = sum(p.numel() for p in model.parameters())
    logger.info(
        "lora_applied",
        trainable_params=trainable,
        total_params=total,
        pct=f"{100 * trainable / total:.2f}%",
    )

    # Prepare dataset
    ds = Dataset.from_list(pairs)

    def tokenize(batch: dict) -> dict:
        return tokenizer(batch["text"], padding="max_length", truncation=True, max_length=64)

    ds = ds.map(tokenize, batched=True)
    ds = ds.rename_column("label", "labels")
    ds.set_format("torch", columns=["input_ids", "attention_mask", "labels"])

    split = ds.train_test_split(test_size=eval_split, seed=42)
    train_ds = split["train"]
    eval_ds = split["test"]

    if len(eval_ds) < MIN_EVAL_SAMPLES:
        return {
            "status": "error",
            "job_id": job_id,
            "message": f"Eval set too small ({len(eval_ds)} < {MIN_EVAL_SAMPLES}). Need more feedback data.",
        }

    # Training
    training_args = TrainingArguments(
        output_dir=str(output_dir / "checkpoints"),
        num_train_epochs=epochs,
        per_device_train_batch_size=batch_size,
        per_device_eval_batch_size=batch_size,
        learning_rate=lr,
        weight_decay=0.01,
        eval_strategy="epoch",
        save_strategy="epoch",
        load_best_model_at_end=True,
        logging_steps=10,
        report_to="none",
        no_cuda=True,  # CPU-only for Intel Mac
        fp16=False,
    )

    trainer = Trainer(
        model=model,
        args=training_args,
        train_dataset=train_ds,
        eval_dataset=eval_ds,
    )

    logger.info("training_started", job_id=job_id, epochs=epochs, train_size=len(train_ds), eval_size=len(eval_ds))
    train_result = trainer.train()

    # Evaluate
    eval_result = trainer.evaluate()
    elapsed = time.time() - t0

    # Save adapted model
    model.save_pretrained(str(output_dir / "adapted_model"))
    tokenizer.save_pretrained(str(output_dir / "adapted_model"))

    # Save job manifest for cloud handoff
    manifest = {
        "job_id": job_id,
        "base_model": base_model_name,
        "lora_config": {
            "r": lora_r,
            "alpha": lora_alpha,
            "dropout": 0.1,
            "target_modules": ["q_lin", "v_lin"],
        },
        "training": {
            "epochs": epochs,
            "lr": lr,
            "batch_size": batch_size,
            "train_samples": len(train_ds),
            "eval_samples": len(eval_ds),
        },
        "metrics": {
            "train_loss": train_result.training_loss,
            "eval_loss": eval_result.get("eval_loss"),
            "eval_accuracy": eval_result.get("eval_accuracy"),
            "elapsed_seconds": round(elapsed, 1),
        },
        "trainable_params": trainable,
        "total_params": total,
        "model_path": str(output_dir / "adapted_model"),
        "data_path": str(output_dir / "training_data.jsonl"),
    }

    (output_dir / "manifest.json").write_text(json.dumps(manifest, indent=2))
    logger.info("finetune_complete", job_id=job_id, elapsed=f"{elapsed:.1f}s", eval_loss=eval_result.get("eval_loss"))

    return {
        "status": "completed",
        "job_id": job_id,
        "manifest": manifest,
    }


# ---------- Status / cancel ----------


async def get_status(job_id: str) -> dict[str, Any]:
    """Get fine-tuning job status by checking manifest."""
    job_dir = Path(f"~/.mindvault/finetune/{job_id}").expanduser()
    manifest_path = job_dir / "manifest.json"

    if manifest_path.exists():
        manifest = json.loads(manifest_path.read_text())
        return {"status": "completed", "job_id": job_id, "manifest": manifest}

    if job_dir.exists():
        return {"status": "in_progress", "job_id": job_id}

    return {"status": "not_found", "job_id": job_id}


async def cancel_finetune(job_id: str) -> dict[str, Any]:
    """Cancel a fine-tuning job (removes job directory)."""
    import shutil

    job_dir = Path(f"~/.mindvault/finetune/{job_id}").expanduser()
    if job_dir.exists():
        shutil.rmtree(job_dir)
        return {"status": "cancelled", "job_id": job_id}
    return {"status": "not_found", "job_id": job_id}


async def list_jobs() -> list[dict[str, Any]]:
    """List all fine-tuning jobs."""
    base = Path("~/.mindvault/finetune").expanduser()
    if not base.exists():
        return []

    jobs = []
    for d in sorted(base.iterdir()):
        if d.is_dir():
            manifest_path = d / "manifest.json"
            if manifest_path.exists():
                manifest = json.loads(manifest_path.read_text())
                jobs.append({"job_id": d.name, "status": "completed", "manifest": manifest})
            else:
                jobs.append({"job_id": d.name, "status": "unknown"})
    return jobs
