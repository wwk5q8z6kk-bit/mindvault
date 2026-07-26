# AI Authority Model

## Rule

AI is a reader and proposer, never a direct canonical writer.

## Typed authorities

| Authority | May do | May not do |
|---|---|---|
| Read | Retrieve authorized canonical/derived data | Expand scope, read secrets, bypass classification |
| Derive | Embed, rank, summarize, extract, detect contradiction | Promote output to fact |
| Propose | Submit typed create/update/delete/link/tag proposals with evidence | Execute the proposal |
| Recommend | Explain options and confidence | Manufacture provenance |
| Approve | Human only by default; narrowly scoped deterministic policy may approve | Be inferred from model confidence |
| Execute | Trusted transaction service after approval | Accept raw model text as an executable mutation |

## Proposal envelope

Every AI-originated proposal contains:

- proposal and actor IDs;
- model/provider/version and local/cloud classification;
- requested operation and typed target;
- base version or content hash;
- evidence IDs and quoted spans;
- visible structured diff;
- confidence and known uncertainty;
- policy decision and required approver;
- creation/expiry timestamps;
- idempotency key;
- approval/rejection record;
- execution transaction and rollback reference.

Approval fails closed if the base version changed, evidence is unavailable, policy changed, or the
executor cannot make the canonical change and audit entry atomically.

## Current incompatibilities

Legacy auto-tagging mutates a node before storage when enabled; sync and some intent/agent paths
write canonical nodes directly. Those paths are disabled or placed behind the proposal gateway
before constitutional compliance can be claimed.
