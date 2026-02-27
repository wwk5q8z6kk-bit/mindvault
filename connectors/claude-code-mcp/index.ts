#!/usr/bin/env npx tsx
/**
 * MindVault MCP Server for Claude Code
 *
 * Stdio-based MCP server that provides Claude Code with access to MindVault.
 *
 * Tools: mindvault_store, mindvault_recall, mindvault_search,
 *        mindvault_forget, mindvault_graph, mindvault_stats
 *
 * Resources: mindvault://knowledge/{id}, mindvault://search?q={query}
 *
 * Configure in ~/.mcp.json:
 * {
 *   "mindvault": {
 *     "command": "npx",
 *     "args": ["tsx", "/path/to/mindvault/connectors/claude-code-mcp/index.ts"],
 *     "env": { "MINDVAULT_URL": "http://localhost:9470" }
 *   }
 * }
 */

import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
  ListResourcesRequestSchema,
  ReadResourceRequestSchema,
} from "@modelcontextprotocol/sdk/types.js";

// --- MindVault Client ---

const MINDVAULT_URL =
  process.env.MINDVAULT_URL || "http://localhost:9470";
const MINDVAULT_TOKEN = process.env.MINDVAULT_TOKEN || "";
const MINDVAULT_NS = process.env.MINDVAULT_NAMESPACE || "claude-code";
const MINDVAULT_TIMEOUT_MS = Number(process.env.MINDVAULT_TIMEOUT_MS || "10000");
const MINDVAULT_MAX_RETRIES = Number(process.env.MINDVAULT_MAX_RETRIES || "2");

interface KnowledgeNode {
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
  };
  metadata: Record<string, unknown>;
}

interface SearchResult {
  node: KnowledgeNode;
  score: number;
  match_source: string;
}

async function mvRequest<T>(
  method: string,
  path: string,
  body?: unknown,
): Promise<T> {
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
  };
  if (MINDVAULT_TOKEN) {
    headers["Authorization"] = `Bearer ${MINDVAULT_TOKEN}`;
  }

  let attempt = 0;
  while (true) {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), MINDVAULT_TIMEOUT_MS);

    try {
      const resp = await fetch(`${MINDVAULT_URL}${path}`, {
        method,
        headers,
        body: body ? JSON.stringify(body) : undefined,
        signal: controller.signal,
      });

      if (resp.ok) {
        return resp.json() as Promise<T>;
      }

      const text = await resp.text().catch(() => "");
      const retryable = resp.status >= 500;
      if (retryable && attempt < MINDVAULT_MAX_RETRIES) {
        attempt += 1;
        await sleep(150 * attempt);
        continue;
      }

      throw new Error(`MindVault ${resp.status}: ${text}`);
    } catch (err) {
      if (attempt >= MINDVAULT_MAX_RETRIES) {
        throw err;
      }
      attempt += 1;
      await sleep(150 * attempt);
    } finally {
      clearTimeout(timer);
    }
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

// --- MCP Server ---

const server = new Server(
  {
    name: "mindvault",
    version: "0.1.0",
  },
  {
    capabilities: {
      tools: {},
      resources: {},
    },
  },
);

// --- Tools ---

server.setRequestHandler(ListToolsRequestSchema, async () => ({
  tools: [
    {
      name: "mindvault_store",
      description:
        "Store knowledge in MindVault. Use to remember facts, decisions, code patterns, project info, or any important information that should persist across sessions.",
      inputSchema: {
        type: "object" as const,
        properties: {
          content: {
            type: "string",
            description: "The knowledge to store",
          },
          kind: {
            type: "string",
            description:
              "Kind: fact, decision, preference, entity, code_snippet, project, conversation, procedure, observation, bookmark",
            default: "fact",
          },
          title: {
            type: "string",
            description: "Short title (optional)",
          },
          tags: {
            type: "string",
            description: "Comma-separated tags",
          },
          importance: {
            type: "number",
            description: "0.0-1.0 importance (default 0.5)",
          },
          namespace: {
            type: "string",
            description: `Namespace (default: ${MINDVAULT_NS})`,
          },
        },
        required: ["content"],
      },
    },
    {
      name: "mindvault_recall",
      description:
        "Recall knowledge from MindVault using semantic search. Returns the most relevant stored memories matching your query. Use hybrid strategy for best results.",
      inputSchema: {
        type: "object" as const,
        properties: {
          query: {
            type: "string",
            description: "What to search for",
          },
          limit: {
            type: "number",
            description: "Max results (default 5)",
          },
          strategy: {
            type: "string",
            description: "Search strategy: hybrid, fulltext, vector, graph",
            default: "hybrid",
          },
          namespace: {
            type: "string",
            description: "Namespace filter",
          },
          kind: {
            type: "string",
            description: "Filter by node kind",
          },
          tags: {
            type: "string",
            description: "Filter by tags (comma-separated)",
          },
        },
        required: ["query"],
      },
    },
    {
      name: "mindvault_search",
      description:
        "Quick full-text search across all stored knowledge. Faster but less semantic than recall.",
      inputSchema: {
        type: "object" as const,
        properties: {
          query: {
            type: "string",
            description: "Search text",
          },
          limit: {
            type: "number",
            description: "Max results (default 10)",
          },
        },
        required: ["query"],
      },
    },
    {
      name: "mindvault_forget",
      description: "Delete a specific knowledge node by ID.",
      inputSchema: {
        type: "object" as const,
        properties: {
          id: {
            type: "string",
            description: "Node ID to delete",
          },
        },
        required: ["id"],
      },
    },
    {
      name: "mindvault_graph",
      description:
        "Explore the knowledge graph — find related nodes connected to a given node.",
      inputSchema: {
        type: "object" as const,
        properties: {
          node_id: {
            type: "string",
            description: "Node ID to explore from",
          },
          depth: {
            type: "number",
            description: "Traversal depth (default 2)",
          },
        },
        required: ["node_id"],
      },
    },
    {
      name: "mindvault_stats",
      description:
        "Show MindVault statistics — node count, health status, version.",
      inputSchema: {
        type: "object" as const,
        properties: {},
      },
    },
  ],
}));

server.setRequestHandler(CallToolRequestSchema, async (request) => {
  const { name, arguments: args } = request.params;

  try {
    switch (name) {
      case "mindvault_store": {
        const a = args as {
          content: string;
          kind?: string;
          title?: string;
          tags?: string;
          importance?: number;
          namespace?: string;
        };
        const node = await mvRequest<KnowledgeNode>("POST", "/api/v1/nodes", {
          kind: a.kind || "fact",
          content: a.content,
          title: a.title,
          namespace: a.namespace || MINDVAULT_NS,
          tags: a.tags ? a.tags.split(",").map((t: string) => t.trim()) : [],
          importance: a.importance ?? 0.5,
          source: "claude-code:mcp",
        });
        return {
          content: [
            {
              type: "text",
              text: `Stored: ${node.id} [${node.kind}]${node.title ? " — " + node.title : ""}\nNamespace: ${node.namespace}${node.tags?.length ? "\nTags: " + node.tags.join(", ") : ""}`,
            },
          ],
        };
      }

      case "mindvault_recall": {
        const a = args as {
          query: string;
          limit?: number;
          strategy?: string;
          namespace?: string;
          kind?: string;
          tags?: string;
        };
        const results = await mvRequest<SearchResult[]>(
          "POST",
          "/api/v1/recall",
          {
            text: a.query,
            strategy: a.strategy || "hybrid",
            limit: a.limit || 5,
            namespace: a.namespace,
            kinds: a.kind ? [a.kind] : undefined,
            tags: a.tags
              ? a.tags.split(",").map((t: string) => t.trim())
              : undefined,
          },
        );

        if (results.length === 0) {
          return {
            content: [{ type: "text", text: "No matching memories found." }],
          };
        }

        const text = results
          .map(
            (r, i) =>
              `${i + 1}. [score: ${r.score.toFixed(3)}] [${r.node.kind}] ${r.node.id}\n   ${r.node.title ? "Title: " + r.node.title + "\n   " : ""}${r.node.content.slice(0, 600)}${r.node.tags?.length ? "\n   Tags: " + r.node.tags.join(", ") : ""}`,
          )
          .join("\n\n");

        return {
          content: [
            {
              type: "text",
              text: `Found ${results.length} result(s):\n\n${text}`,
            },
          ],
        };
      }

      case "mindvault_search": {
        const a = args as { query: string; limit?: number };
        const params = new URLSearchParams({
          q: a.query,
          limit: String(a.limit || 10),
          type: "fulltext",
        });
        const results = await mvRequest<SearchResult[]>(
          "GET",
          `/api/v1/search?${params}`,
        );

        if (results.length === 0) {
          return {
            content: [{ type: "text", text: "No results found." }],
          };
        }

        const text = results
          .map(
            (r, i) =>
              `${i + 1}. [${r.score.toFixed(3)}] ${r.node.id} [${r.node.kind}] ${r.node.content.slice(0, 200)}`,
          )
          .join("\n");

        return {
          content: [{ type: "text", text: `${results.length} result(s):\n${text}` }],
        };
      }

      case "mindvault_forget": {
        const a = args as { id: string };
        const result = await mvRequest<{ deleted: boolean }>(
          "DELETE",
          `/api/v1/nodes/${a.id}`,
        );
        return {
          content: [
            {
              type: "text",
              text: result.deleted
                ? `Deleted: ${a.id}`
                : `Not found: ${a.id}`,
            },
          ],
        };
      }

      case "mindvault_graph": {
        const a = args as { node_id: string; depth?: number };
        const neighbors = await mvRequest<string[]>(
          "GET",
          `/api/v1/graph/neighbors/${a.node_id}?depth=${a.depth || 2}`,
        );

        if (neighbors.length === 0) {
          return {
            content: [
              {
                type: "text",
                text: `No neighbors found for ${a.node_id} within depth ${a.depth || 2}`,
              },
            ],
          };
        }

        // Fetch details for each neighbor
        const details: string[] = [];
        for (const nid of neighbors.slice(0, 10)) {
          try {
            const node = await mvRequest<KnowledgeNode | null>(
              "GET",
              `/api/v1/nodes/${nid}`,
            );
            if (node) {
              details.push(
                `  ${nid} [${node.kind}] ${node.title || node.content.slice(0, 100)}`,
              );
            } else {
              details.push(`  ${nid} (not found)`);
            }
          } catch {
            details.push(`  ${nid} (error)`);
          }
        }

        return {
          content: [
            {
              type: "text",
              text: `Neighbors of ${a.node_id} (depth ${a.depth || 2}):\n${details.join("\n")}${neighbors.length > 10 ? `\n  ... and ${neighbors.length - 10} more` : ""}`,
            },
          ],
        };
      }

      case "mindvault_stats": {
        const health = await mvRequest<{
          status: string;
          node_count: number;
          version: string;
        }>("GET", "/api/v1/health");

        return {
          content: [
            {
              type: "text",
              text: `MindVault Status:\n  Status: ${health.status}\n  Nodes: ${health.node_count}\n  Version: ${health.version}\n  Server: ${MINDVAULT_URL}`,
            },
          ],
        };
      }

      default:
        return {
          content: [{ type: "text", text: `Unknown tool: ${name}` }],
          isError: true,
        };
    }
  } catch (err) {
    return {
      content: [
        {
          type: "text",
          text: `Error: ${err instanceof Error ? err.message : String(err)}`,
        },
      ],
      isError: true,
    };
  }
});

// --- Resources ---

server.setRequestHandler(ListResourcesRequestSchema, async () => ({
  resources: [
    {
      uri: "mindvault://stats",
      name: "MindVault Stats",
      description: "Current MindVault statistics and health",
      mimeType: "application/json",
    },
  ],
}));

server.setRequestHandler(ReadResourceRequestSchema, async (request) => {
  const { uri } = request.params;

  if (uri === "mindvault://stats") {
    try {
      const health = await mvRequest<{
        status: string;
        node_count: number;
        version: string;
      }>("GET", "/api/v1/health");
      return {
        contents: [
          {
            uri,
            mimeType: "application/json",
            text: JSON.stringify(health, null, 2),
          },
        ],
      };
    } catch (err) {
      return {
        contents: [
          {
            uri,
            mimeType: "text/plain",
            text: `Error: ${err instanceof Error ? err.message : String(err)}`,
          },
        ],
      };
    }
  }

  // mindvault://knowledge/{id}
  const knowledgeMatch = uri.match(/^mindvault:\/\/knowledge\/(.+)$/);
  if (knowledgeMatch) {
    const id = knowledgeMatch[1];
    try {
      const node = await mvRequest<KnowledgeNode | null>(
        "GET",
        `/api/v1/nodes/${id}`,
      );
      return {
        contents: [
          {
            uri,
            mimeType: "application/json",
            text: JSON.stringify(node, null, 2),
          },
        ],
      };
    } catch (err) {
      return {
        contents: [
          {
            uri,
            mimeType: "text/plain",
            text: `Error: ${err instanceof Error ? err.message : String(err)}`,
          },
        ],
      };
    }
  }

  // mindvault://search?q={query}
  const searchMatch = uri.match(/^mindvault:\/\/search\?q=(.+)$/);
  if (searchMatch) {
    const query = decodeURIComponent(searchMatch[1]);
    try {
      const results = await mvRequest<SearchResult[]>(
        "POST",
        "/api/v1/recall",
        {
          text: query,
          strategy: "hybrid",
          limit: 10,
        },
      );
      return {
        contents: [
          {
            uri,
            mimeType: "application/json",
            text: JSON.stringify(results, null, 2),
          },
        ],
      };
    } catch (err) {
      return {
        contents: [
          {
            uri,
            mimeType: "text/plain",
            text: `Error: ${err instanceof Error ? err.message : String(err)}`,
          },
        ],
      };
    }
  }

  return {
    contents: [
      {
        uri,
        mimeType: "text/plain",
        text: `Unknown resource: ${uri}`,
      },
    ],
  };
});

// --- Start ---

async function main() {
  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error("MindVault MCP server started");
}

main().catch((err) => {
  console.error("Fatal:", err);
  process.exit(1);
});
