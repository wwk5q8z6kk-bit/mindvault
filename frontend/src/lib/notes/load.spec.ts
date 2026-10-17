import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { KnowledgeNode } from '$lib/api/types';

const listNodesMock = vi.fn();

vi.mock('$lib/api/nodes', () => ({
	listNodes: (...args: unknown[]) => listNodesMock(...args)
}));

function makeNode(id: string, kind: KnowledgeNode['kind']): KnowledgeNode {
	return {
		id,
		kind,
		title: id,
		content: '',
		source: null,
		namespace: null,
		tags: [],
		importance: 0,
		temporal: {
			created_at: '2026-01-01T00:00:00Z',
			updated_at: '2026-01-01T00:00:00Z'
		},
		metadata: {}
	};
}

describe('listNodesForKinds', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	it('starts all kind requests together and preserves the requested kind order', async () => {
		let resolveFact: (nodes: KnowledgeNode[]) => void;
		let resolveDecision: (nodes: KnowledgeNode[]) => void;
		const factNodes = new Promise<KnowledgeNode[]>((resolve) => {
			resolveFact = resolve;
		});
		const decisionNodes = new Promise<KnowledgeNode[]>((resolve) => {
			resolveDecision = resolve;
		});
		listNodesMock.mockImplementation(({ kind }: { kind: string }) =>
			kind === 'fact' ? factNodes : decisionNodes
		);

		const { listNodesForKinds } = await import('./load');
		const result = listNodesForKinds(['fact', 'decision']);

		expect(listNodesMock).toHaveBeenCalledTimes(2);
		expect(listNodesMock).toHaveBeenNthCalledWith(1, { kind: 'fact', limit: 100 });
		expect(listNodesMock).toHaveBeenNthCalledWith(2, { kind: 'decision', limit: 100 });

		resolveDecision!([makeNode('decision-1', 'decision')]);
		resolveFact!([makeNode('fact-1', 'fact')]);

		await expect(result).resolves.toEqual([
			makeNode('fact-1', 'fact'),
			makeNode('decision-1', 'decision')
		]);
	});

	it('continues loading other kinds when one request fails', async () => {
		listNodesMock
			.mockResolvedValueOnce([makeNode('fact-1', 'fact')])
			.mockRejectedValueOnce(new Error('unavailable'));

		const { listNodesForKinds } = await import('./load');

		await expect(listNodesForKinds(['fact', 'decision'])).resolves.toEqual([
			makeNode('fact-1', 'fact')
		]);
	});
});
