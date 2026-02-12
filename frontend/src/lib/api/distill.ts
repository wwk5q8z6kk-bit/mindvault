import { fetchJson } from './client';

export type DistillRequest =
	| {
			kind: 'namespace';
			namespace: string;
			max_nodes?: number;
	  }
	| {
			kind: 'temporal';
			days?: number;
	  }
	| {
			kind: 'topic_deep_dive';
			topic: string;
	  };

export interface DistillResult {
	kind: string;
	title: string;
	summary: string;
	source_count: number;
	generated_at: string;
}

export async function runDistill(request: DistillRequest): Promise<DistillResult> {
	return await fetchJson<DistillResult>('/api/v1/distill', {
		method: 'POST',
		body: JSON.stringify(request)
	});
}
