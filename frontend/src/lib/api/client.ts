import { markApiFailure, markApiSuccess } from '$lib/stores/api-health';
export const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? 'http://127.0.0.1:9470';

export class ApiError extends Error {
	status: number;
	body?: unknown;

	constructor(message: string, status: number, body?: unknown) {
		super(message);
		this.status = status;
		this.body = body;
	}
}

export async function fetchJson<T>(
	path: string,
	options: RequestInit = {},
	{ timeoutMs = 10000 }: { timeoutMs?: number } = {}
): Promise<T> {
	const controller = new AbortController();
	const timer = setTimeout(() => controller.abort(), timeoutMs);

	try {
		let res: Response;
		try {
			res = await fetch(`${API_BASE_URL}${path}`, {
				...options,
				headers: {
					'Content-Type': 'application/json',
					...(options.headers ?? {})
				},
				signal: controller.signal
			});
		} catch (err) {
			const kind = err instanceof Error && err.name === 'AbortError' ? 'timeout' : 'network';
			const detail = err instanceof Error ? err.message : 'Request failed';
			markApiFailure(kind, path, detail);
			throw err;
		}

		if (!res.ok) {
			let body: unknown = undefined;
			try {
				body = await res.json();
			} catch {
				body = await res.text();
			}
			if (res.status >= 500) {
				markApiFailure('server', path, `HTTP ${res.status}`);
			} else {
				// 4xx responses still prove the backend is reachable.
				markApiSuccess(path);
			}
			throw new ApiError(`Request failed (${res.status})`, res.status, body);
		}

		if (res.status === 204 || res.status === 205) {
			markApiSuccess(path);
			return undefined as T;
		}

		const text = await res.text();
		if (!text) {
			markApiSuccess(path);
			return undefined as T;
		}

		try {
			const data = JSON.parse(text) as T;
			markApiSuccess(path);
			return data;
		} catch {
			markApiFailure('invalid_response', path, 'Invalid JSON response');
			throw new ApiError('Invalid JSON response', res.status, text);
		}
	} finally {
		clearTimeout(timer);
	}
}
