"""Fine-tuning API endpoints for AgentEvolver cloud-train PoC."""

from __future__ import annotations

from typing import Any

from fastapi import APIRouter
from pydantic import BaseModel, Field

from mindvault_ai.pipelines.finetune import (
    cancel_finetune,
    export_feedback,
    feedback_to_training_pairs,
    get_status,
    is_available,
    list_jobs,
    start_finetune,
)

router = APIRouter(prefix="/v1/finetune", tags=["finetune"])


class FinetuneRequest(BaseModel):
    vault_url: str = Field(default="http://127.0.0.1:9470")
    auth_token: str | None = None
    intent_type: str | None = None
    output_dir: str = Field(default="~/.mindvault/finetune")
    epochs: int = Field(default=3, ge=1, le=20)
    lr: float = Field(default=1e-4, gt=0, le=1.0)
    lora_r: int = Field(default=8, ge=2, le=64)
    lora_alpha: int = Field(default=16, ge=4, le=128)
    batch_size: int = Field(default=4, ge=1, le=32)
    eval_split: float = Field(default=0.2, gt=0, lt=1.0)


class ExportRequest(BaseModel):
    vault_url: str = Field(default="http://127.0.0.1:9470")
    auth_token: str | None = None
    intent_type: str | None = None
    limit: int = Field(default=500, ge=1, le=5000)


@router.get("/status")
async def finetune_status_overview() -> dict[str, Any]:
    """Check if fine-tuning is available and list recent jobs."""
    available = is_available()
    jobs = await list_jobs() if available else []
    return {
        "available": available,
        "install_hint": "uv pip install 'mindvault-ai[finetune]'" if not available else None,
        "jobs": jobs,
    }


@router.post("/export")
async def export_training_data(req: ExportRequest) -> dict[str, Any]:
    """Export feedback records and convert to training pairs."""
    raw = await export_feedback(
        vault_url=req.vault_url,
        limit=req.limit,
        intent_type=req.intent_type,
        auth_token=req.auth_token,
    )
    pairs = feedback_to_training_pairs(raw)
    return {
        "raw_count": len(raw),
        "pair_count": len(pairs),
        "pairs": pairs[:50],  # Preview first 50
        "label_distribution": {
            "applied": sum(1 for p in pairs if p["label"] == 1),
            "dismissed": sum(1 for p in pairs if p["label"] == 0),
        },
    }


@router.post("/start")
async def start_finetune_job(req: FinetuneRequest) -> dict[str, Any]:
    """Start a LoRA fine-tuning job on feedback data."""
    return await start_finetune(req.model_dump())


@router.get("/jobs/{job_id}")
async def get_job_status(job_id: str) -> dict[str, Any]:
    """Get fine-tuning job status."""
    return await get_status(job_id)


@router.delete("/jobs/{job_id}")
async def cancel_job(job_id: str) -> dict[str, Any]:
    """Cancel or delete a fine-tuning job."""
    return await cancel_finetune(job_id)
