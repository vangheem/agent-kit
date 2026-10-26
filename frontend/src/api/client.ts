import { apiUrl } from "../config";

export class ApiError extends Error {
  status: number;
  code: string;

  constructor(message: string, status: number, code: string) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.code = code;
  }
}

export type RequestAccess = "public" | "protected" | "auth-bootstrap";

type RequestOptions = {
  method?: string;
  body?: unknown;
  access?: RequestAccess;
  signal?: AbortSignal;
  headers?: HeadersInit;
};

type UnauthorizedHandler = () => void | Promise<void>;

let unauthorizedHandler: UnauthorizedHandler | null = null;
let handlingUnauthorized = false;

export function setUnauthorizedHandler(
  handler: UnauthorizedHandler | null,
): void {
  unauthorizedHandler = handler;
}

export function errorText(error: unknown): string {
  if (error instanceof Error && error.message.length > 0) {
    return error.message;
  }
  return "Request failed";
}

async function notifyUnauthorized(access: RequestAccess): Promise<void> {
  if (
    access === "public" ||
    access === "auth-bootstrap" ||
    !unauthorizedHandler ||
    handlingUnauthorized
  ) {
    return;
  }
  handlingUnauthorized = true;
  try {
    await unauthorizedHandler();
  } finally {
    handlingUnauthorized = false;
  }
}

async function parseError(response: Response): Promise<ApiError> {
  try {
    const body = (await response.json()) as {
      error?: { code?: unknown; message?: unknown };
    };
    const message =
      typeof body.error?.message === "string" && body.error.message.length > 0
        ? body.error.message
        : `HTTP ${response.status}`;
    const code =
      typeof body.error?.code === "string" && body.error.code.length > 0
        ? body.error.code
        : "http_error";
    return new ApiError(message, response.status, code);
  } catch {
    return new ApiError(
      `HTTP ${response.status}`,
      response.status,
      "http_error",
    );
  }
}

export async function apiRequest<T>(
  path: string,
  options: RequestOptions = {},
): Promise<T> {
  const access = options.access ?? "protected";
  const headers = new Headers(options.headers);
  let body: string | undefined;
  if (options.body !== undefined) {
    headers.set("Content-Type", "application/json");
    body = JSON.stringify(options.body);
  }

  const response = await fetch(`${apiUrl}${path}`, {
    method: options.method,
    headers,
    body,
    credentials: "include",
    signal: options.signal,
  });

  if (!response.ok) {
    if (response.status === 401) {
      await notifyUnauthorized(access);
    }
    throw await parseError(response);
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return (await response.json()) as T;
}
