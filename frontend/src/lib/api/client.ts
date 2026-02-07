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
		const res = await fetch(`${API_BASE_URL}${path}`, {
			...options,
			headers: {
				'Content-Type': 'application/json',
				...(options.headers ?? {})
			},
			signal: controller.signal
		});

		if (!res.ok) {
			let body: unknown = undefined;
			try {
				body = await res.json();
			} catch {
				body = await res.text();
			}
			throw new ApiError(`Request failed (${res.status})`, res.status, body);
		}

		return (await res.json()) as T;
	} finally {
		clearTimeout(timer);
	}
}
