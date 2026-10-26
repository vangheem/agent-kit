import type { ServiceConnection } from "../types";
import { apiRequest } from "./client";

export type ServiceInput = {
  provider: string;
  credentials: Record<string, unknown>;
};

export function list() {
  return apiRequest<{ services: ServiceConnection[] }>("/api/services");
}

export function save(kind: string, body: ServiceInput) {
  return apiRequest<ServiceConnection>(
    `/api/services/${encodeURIComponent(kind)}`,
    {
      method: "PUT",
      body,
    },
  );
}

export function remove(kind: string) {
  return apiRequest<void>(`/api/services/${encodeURIComponent(kind)}`, {
    method: "DELETE",
  });
}

export function parseCredentials(raw: string): Record<string, unknown> {
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    throw new Error("Credentials must be a JSON object");
  }
  if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) {
    throw new Error("Credentials must be a JSON object");
  }
  return parsed as Record<string, unknown>;
}
