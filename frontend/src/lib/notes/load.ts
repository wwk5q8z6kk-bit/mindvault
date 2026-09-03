import { listNodes } from '$lib/api/nodes';
import type { KnowledgeNode, NodeKind } from '$lib/api/types';

export async function listNodesForKinds(
	kinds: readonly NodeKind[],
	limit = 100
): Promise<KnowledgeNode[]> {
	const nodesByKind = await Promise.all(
		kinds.map(async (kind) => {
			try {
				return await listNodes({ kind, limit });
			} catch {
				return [];
			}
		})
	);

	return nodesByKind.flat();
}
