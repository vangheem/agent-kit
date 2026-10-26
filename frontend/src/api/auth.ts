import type { User } from "../types";
import { apiRequest } from "./client";

export type Credentials = {
  email: string;
  password: string;
};

export function login(body: Credentials) {
  return apiRequest<User>("/api/auth/login", {
    method: "POST",
    body,
    access: "public",
  });
}

export function register(body: Credentials) {
  return apiRequest<User>("/api/auth/register", {
    method: "POST",
    body,
    access: "public",
  });
}

export function logout() {
  return apiRequest<void>("/api/auth/logout", {
    method: "POST",
    access: "public",
  });
}

export function me(options?: { signal?: AbortSignal }) {
  return apiRequest<User>("/api/auth/me", {
    access: "auth-bootstrap",
    signal: options?.signal,
  });
}
