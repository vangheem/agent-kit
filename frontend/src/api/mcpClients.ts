import type { CreatedMcpClient, McpClient } from "../types";
import { apiRequest } from "./client";

export type McpClientInput = {
  name: string;
  enabled_actions: string[];
};

export function list() {
  return apiRequest<{ clients: McpClient[] }>("/api/mcp-clients");
}

export function create(body: McpClientInput) {
  return apiRequest<CreatedMcpClient>("/api/mcp-clients", {
    method: "POST",
    body,
  });
}

export function revoke(id: string) {
  return apiRequest<void>(`/api/mcp-clients/${encodeURIComponent(id)}`, {
    method: "DELETE",
  });
}
