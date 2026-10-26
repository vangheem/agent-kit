import type { ActionDescriptor } from "../types";
import { apiRequest } from "./client";

export function list() {
  return apiRequest<{ actions: ActionDescriptor[] }>("/api/actions");
}
