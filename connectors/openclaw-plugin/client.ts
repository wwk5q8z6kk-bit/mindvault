/**
 * KnowledgeVault REST API client.
 * Connects via HTTP or Unix socket to the KnowledgeVault server.
 */

export interface KnowledgeVaultConfig {
  serverUrl: string;
  socketPath?: string;
  authToken?: string;
  namespace: string;
  requestTimeoutMs?: number;
  maxRetries?: number;
}

export interface KnowledgeNode {
  id: string;
  kind: string;
  title?: string;
  content: string;
  source?: string;
  namespace: string;
  tags: string[];
  importance: number;
  temporal: {
    created_at: string;
    updated_at: string;
    last_accessed_at: string;
    access_count: number;
    version: number;
    expires_at?: string;
  };
  metadata: Record<string, unknown>;
}

export interface SearchResult {
  node: KnowledgeNode;
  score: number;
  match_source: string;
}

export interface StoreRequest {
  kind: string;
  content: string;
  title?: string;
  source?: string;
  namespace?: string;
  tags?: string[];
  importance?: number;
}

export interface RecallRequest {
  text: string;
  strategy?: string;
  limit?: number;
  min_score?: number;
  namespace?: string;
  kinds?: string[];
  tags?: string[];
}

export class KnowledgeVaultClient {
  private baseUrl: string;
  private headers: Record<string, string>;
  private requestTimeoutMs: number;
  private maxRetries: number;

  constructor(private config: KnowledgeVaultConfig) {
    this.baseUrl = config.serverUrl.replace(/\/$/, "");
    this.headers = { "Content-Type": "application/json" };
    this.requestTimeoutMs = config.requestTimeoutMs ?? 10_000;
    this.maxRetries = config.maxRetries ?? 2;
    if (config.authToken) {
      this.headers["Authorization"] = `Bearer ${config.authToken}`;
    }
  }

  private async request<T>(
    method: string,
    path: string,
    body?: unknown,
  ): Promise<T> {
    const url = `${this.baseUrl}${path}`;
    let attempt = 0;
    while (true) {
      const controller = new AbortController();
      const timer = setTimeout(() => controller.abort(), this.requestTimeoutMs);

      try {
        const opts: RequestInit = {
          method,
          headers: this.headers,
          signal: controller.signal,
        };
        if (body) {
          opts.body = JSON.stringify(body);
        }

        const resp = await fetch(url, opts);
        if (resp.ok) {
          return resp.json() as Promise<T>;
        }

        const text = await resp.text().catch(() => "");
        const isRetryable = resp.status >= 500;
        if (isRetryable && attempt < this.maxRetries) {
          attempt += 1;
          await sleep(150 * attempt);
          continue;
        }

        throw new Error(
          `KnowledgeVault API error: ${resp.status} ${resp.statusText} — ${text}`,
        );
      } catch (err) {
        if (attempt >= this.maxRetries) {
          throw err;
        }
        attempt += 1;
        await sleep(150 * attempt);
      } finally {
        clearTimeout(timer);
      }
    }
  }

  async health(): Promise<{
    status: string;
    node_count: number;
    version: string;
  }> {
    return this.request("GET", "/api/v1/health");
  }

  async store(req: StoreRequest): Promise<KnowledgeNode> {
    if (!req.namespace) {
      req.namespace = this.config.namespace;
    }
    return this.request("POST", "/api/v1/nodes", req);
  }

  async get(id: string): Promise<KnowledgeNode | null> {
    return this.request("GET", `/api/v1/nodes/${id}`);
  }

  async recall(req: RecallRequest): Promise<SearchResult[]> {
    return this.request("POST", "/api/v1/recall", req);
  }

  async search(
    query: string,
    limit = 10,
    type = "hybrid",
  ): Promise<SearchResult[]> {
    const params = new URLSearchParams({
      q: query,
      limit: String(limit),
      type,
    });
    return this.request("GET", `/api/v1/search?${params}`);
  }

  async deleteNode(id: string): Promise<{ deleted: boolean }> {
    return this.request("DELETE", `/api/v1/nodes/${id}`);
  }

  async listNodes(opts?: {
    namespace?: string;
    kind?: string;
    limit?: number;
    offset?: number;
  }): Promise<KnowledgeNode[]> {
    const params = new URLSearchParams();
    if (opts?.namespace) params.set("namespace", opts.namespace);
    if (opts?.kind) params.set("kind", opts.kind);
    if (opts?.limit) params.set("limit", String(opts.limit));
    if (opts?.offset) params.set("offset", String(opts.offset));
    return this.request("GET", `/api/v1/nodes?${params}`);
  }

  async addRelationship(
    fromNode: string,
    toNode: string,
    kind: string,
    weight = 1.0,
  ): Promise<{ id: string }> {
    return this.request("POST", "/api/v1/graph/relationships", {
      from_node: fromNode,
      to_node: toNode,
      kind,
      weight,
    });
  }

  async getNeighbors(
    nodeId: string,
    depth = 2,
  ): Promise<string[]> {
    return this.request(
      "GET",
      `/api/v1/graph/neighbors/${nodeId}?depth=${depth}`,
    );
  }

  async isAvailable(): Promise<boolean> {
    try {
      await this.health();
      return true;
    } catch {
      return false;
    }
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
