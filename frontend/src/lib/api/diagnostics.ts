import { API_BASE_URL, fetchJson } from './client';

export interface HealthDiagnostics {
	status: 'healthy' | 'degraded' | string;
	database: {
		status: 'ok' | 'error' | string;
		latency_ms: number;
	};
	uptime_seconds: number;
	version: string;
}

export interface EmbeddingDiagnostics {
	configured_provider: string;
	configured_model: string;
	configured_dimensions: number;
	effective_provider: string;
	effective_model: string;
	effective_dimensions: number;
	fallback_to_noop: boolean;
	reason?: string | null;
	local_embeddings_feature_enabled: boolean;
}

export interface HistogramStats {
	count: number;
	min?: number | null;
	max?: number | null;
	mean?: number | null;
	p50?: number | null;
	p95?: number | null;
	p99?: number | null;
}

export interface PerformanceDiagnostics {
	health_check_latency_ms: HistogramStats;
	uptime_seconds: number;
}

export async function getHealthDiagnostics(): Promise<HealthDiagnostics> {
	return await fetchJson<HealthDiagnostics>('/api/v1/diagnostics/health');
}

export async function getEmbeddingDiagnostics(): Promise<EmbeddingDiagnostics> {
	return await fetchJson<EmbeddingDiagnostics>('/api/v1/diagnostics/embedding');
}

export async function getPerformanceDiagnostics(): Promise<PerformanceDiagnostics> {
	return await fetchJson<PerformanceDiagnostics>('/api/v1/diagnostics/performance');
}

export async function getPrometheusMetrics(): Promise<string> {
	const res = await fetch(`${API_BASE_URL}/metrics`);
	if (!res.ok) {
		throw new Error(`metrics request failed (${res.status})`);
	}
	return await res.text();
}
