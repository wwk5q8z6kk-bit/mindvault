/**
 * MindVault — OpenClaw Memory Plugin
 *
 * Drop-in replacement for memory-lancedb.
 * Connects to MindVault server via REST API.
 *
 * Configure in openclaw.json:
 *   "plugins.slots.memory": "mindvault"
 *   "plugins.entries.mindvault.config": { ... }
 */

import {
  MindVaultClient,
  type MindVaultConfig,
  type RecallRequest,
  type StoreRequest,
} from "./client.ts";

interface PluginApi {
  pluginConfig?: Record<string, unknown>;
  log?: {
    info?: (msg: string) => void;
    warn?: (msg: string) => void;
    error?: (msg: string) => void;
    debug?: (msg: string) => void;
  };
  logger?: {
    info: (msg: string) => void;
    warn: (msg: string) => void;
    error: (msg: string) => void;
    debug?: (msg: string) => void;
  };
  registerTool?: (tool: unknown, opts?: { name?: string }) => void;
  on?: (
    hookName: string,
    handler: (...args: unknown[]) => unknown,
    opts?: { priority?: number },
  ) => void;
  registerHook?: (
    events: string | string[],
    handler: (...args: unknown[]) => unknown,
  ) => void;
  registerService?: (service: {
    id: string;
    start: () => void;
    stop: () => void;
  }) => void;
}

function getLog(api: PluginApi) {
  const logger = api.logger ?? api.log;
  return {
    info: logger?.info ?? (() => {}),
    warn: logger?.warn ?? (() => {}),
    error: logger?.error ?? (() => {}),
    debug: logger?.debug ?? (() => {}),
  };
}

export default function mindvaultPlugin(api: PluginApi) {
  const log = getLog(api);
  const cfg = api.pluginConfig ?? {};

  // Parse config
  const mvConfig: MindVaultConfig = {
    serverUrl: (cfg.serverUrl as string) || "http://localhost:9470",
    socketPath: cfg.socketPath as string | undefined,
    authToken: cfg.authToken as string | undefined,
    namespace: (cfg.namespace as string) || "openclaw",
    requestTimeoutMs: (cfg.requestTimeoutMs as number) || 10_000,
    maxRetries: (cfg.maxRetries as number) || 2,
  };

  const autoCapture = cfg.autoCapture === true; // default false (privacy-safe)
  const autoRecall = cfg.autoRecall !== false; // default true
  const recallLimit = (cfg.recallLimit as number) || 5;
  const recallStrategy = (cfg.recallStrategy as string) || "hybrid";

  const client = new MindVaultClient(mvConfig);

  log.info("[mindvault] plugin initializing");

  // --- Register Service ---
  api.registerService?.({
    id: "mindvault",
    start: () => log.info("[mindvault] service started"),
    stop: () => log.info("[mindvault] service stopped"),
  });

  // --- Hook: before_agent_start (auto-recall) ---
  const registerHook =
    api.on ??
    ((hookName: string, handler: (...args: unknown[]) => unknown) => {
      api.registerHook?.(hookName, handler);
    });

  if (autoRecall) {
    registerHook(
      "before_agent_start",
      async (event: unknown) => {
        const ev = event as { prompt?: string };
        const prompt = ev?.prompt;
        if (!prompt || prompt.length < 5) return;

        try {
          const available = await client.isAvailable();
          if (!available) {
            log.debug(
              "[mindvault] server not available, skipping auto-recall",
            );
            return;
          }

          const results = await client.recall({
            text: prompt,
            strategy: recallStrategy,
            limit: recallLimit,
            namespace: mvConfig.namespace,
          });

          if (results.length === 0) return;

          const memories = results
            .map(
              (r) =>
                `- [${r.node.kind}] ${r.node.title ? r.node.title + ": " : ""}${r.node.content.slice(0, 500)}`,
            )
            .join("\n");

          const context = `<mindvault-memories>\nRelevant knowledge from MindVault (${results.length} results):\n${memories}\n</mindvault-memories>`;

          log.info(
            `[mindvault] recalled ${results.length} memories for agent context`,
          );

          return { prependContext: context };
        } catch (err) {
          log.warn(
            `[mindvault] auto-recall failed: ${err instanceof Error ? err.message : String(err)}`,
          );
        }
      },
      { priority: 50 },
    );
  }

  // --- Hook: agent_end (auto-capture) ---
  if (autoCapture) {
    registerHook("agent_end", async (event: unknown) => {
      const ev = event as {
        messages?: Array<{ role?: string; content?: string }>;
        success?: boolean;
      };

      if (!ev?.success || !ev?.messages) return;

      try {
        const available = await client.isAvailable();
        if (!available) return;

        // Extract key facts from the conversation
        // Simple heuristic: capture assistant messages that look like conclusions/facts
        const assistantMessages = (ev.messages ?? []).filter(
          (m) => m.role === "assistant" && m.content && m.content.length > 20,
        );

        if (assistantMessages.length === 0) return;

        // Store a summary of the last substantial assistant message
        const lastMsg = assistantMessages[assistantMessages.length - 1];
        const content = lastMsg.content!;

        // Only store if it looks like substantive content (not just greetings)
        if (content.length < 50) return;

        // Truncate very long messages
        const truncated =
          content.length > 2000 ? content.slice(0, 2000) + "..." : content;

        await client.store({
          kind: "conversation",
          content: truncated,
          source: "openclaw:auto-capture",
          namespace: mvConfig.namespace,
          tags: ["auto-captured", "openclaw"],
          importance: 0.3,
        });

        log.debug("[mindvault] auto-captured conversation knowledge");
      } catch (err) {
        log.debug(
          `[mindvault] auto-capture failed: ${err instanceof Error ? err.message : String(err)}`,
        );
      }
    });
  }

  // --- Tool: memory_store ---
  api.registerTool?.(
    {
      name: "memory_store",
      label: "MindVault Store",
      description:
        "Store a piece of knowledge in MindVault. Use this to remember facts, decisions, preferences, or any important information.",
      parameters: {
        type: "object",
        properties: {
          content: {
            type: "string",
            description: "The knowledge content to store",
          },
          kind: {
            type: "string",
            description:
              "Node kind: fact, decision, preference, entity, code_snippet, project, conversation, procedure, observation, bookmark",
            default: "fact",
          },
          title: {
            type: "string",
            description: "Optional title for the knowledge",
          },
          tags: {
            type: "string",
            description: "Comma-separated tags",
          },
          importance: {
            type: "number",
            description: "Importance score 0.0-1.0 (default 0.5)",
          },
        },
        required: ["content"],
      },
      async execute(
        _toolCallId: string,
        params: {
          content: string;
          kind?: string;
          title?: string;
          tags?: string;
          importance?: number;
        },
      ) {
        try {
          const req: StoreRequest = {
            kind: params.kind || "fact",
            content: params.content,
            title: params.title,
            namespace: mvConfig.namespace,
            tags: params.tags
              ? params.tags.split(",").map((t) => t.trim())
              : [],
            importance: params.importance ?? 0.5,
            source: "openclaw:tool",
          };

          const node = await client.store(req);

          return {
            content: [
              {
                type: "text",
                text: `Stored in MindVault: ${node.id} (${node.kind})${node.title ? " — " + node.title : ""}`,
              },
            ],
          };
        } catch (err) {
          return {
            content: [
              {
                type: "text",
                text: `Failed to store: ${err instanceof Error ? err.message : String(err)}`,
              },
            ],
            isError: true,
          };
        }
      },
    },
    { name: "memory_store" },
  );

  // --- Tool: memory_search ---
  api.registerTool?.(
    {
      name: "memory_search",
      label: "MindVault Search",
      description:
        "Search knowledge stored in MindVault. Returns relevant memories matching the query.",
      parameters: {
        type: "object",
        properties: {
          query: {
            type: "string",
            description: "Search query text",
          },
          limit: {
            type: "number",
            description: "Max results (default 5)",
          },
          strategy: {
            type: "string",
            description: "Search strategy: hybrid, fulltext, vector, graph",
          },
          kind: {
            type: "string",
            description: "Filter by node kind",
          },
        },
        required: ["query"],
      },
      async execute(
        _toolCallId: string,
        params: {
          query: string;
          limit?: number;
          strategy?: string;
          kind?: string;
        },
      ) {
        try {
          const req: RecallRequest = {
            text: params.query,
            strategy: params.strategy || "hybrid",
            limit: params.limit || 5,
            namespace: mvConfig.namespace,
          };
          if (params.kind) {
            req.kinds = [params.kind];
          }

          const results = await client.recall(req);

          if (results.length === 0) {
            return {
              content: [{ type: "text", text: "No matching memories found." }],
            };
          }

          const formatted = results
            .map(
              (r, i) =>
                `${i + 1}. [${r.score.toFixed(3)}] [${r.node.kind}] ${r.node.title ? r.node.title + ": " : ""}${r.node.content.slice(0, 500)}${r.node.tags.length > 0 ? "\n   tags: " + r.node.tags.join(", ") : ""}`,
            )
            .join("\n\n");

          return {
            content: [
              {
                type: "text",
                text: `Found ${results.length} memories:\n\n${formatted}`,
              },
            ],
          };
        } catch (err) {
          return {
            content: [
              {
                type: "text",
                text: `Search failed: ${err instanceof Error ? err.message : String(err)}`,
              },
            ],
            isError: true,
          };
        }
      },
    },
    { name: "memory_search" },
  );

  // --- Tool: memory_forget ---
  api.registerTool?.(
    {
      name: "memory_forget",
      label: "MindVault Forget",
      description: "Delete a knowledge node from MindVault by its ID.",
      parameters: {
        type: "object",
        properties: {
          id: {
            type: "string",
            description: "The node ID to delete",
          },
        },
        required: ["id"],
      },
      async execute(_toolCallId: string, params: { id: string }) {
        try {
          const result = await client.deleteNode(params.id);
          return {
            content: [
              {
                type: "text",
                text: result.deleted
                  ? `Deleted memory: ${params.id}`
                  : `Memory not found: ${params.id}`,
              },
            ],
          };
        } catch (err) {
          return {
            content: [
              {
                type: "text",
                text: `Delete failed: ${err instanceof Error ? err.message : String(err)}`,
              },
            ],
            isError: true,
          };
        }
      },
    },
    { name: "memory_forget" },
  );

  log.info(
    `[mindvault] plugin ready (autoCapture=${autoCapture}, autoRecall=${autoRecall}, server=${mvConfig.serverUrl})`,
  );
}
