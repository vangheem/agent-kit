import { afterEach, describe, expect, it, vi } from "vitest";

import { apiRequest, setUnauthorizedHandler } from "./client";

describe("apiRequest", () => {
  afterEach(() => {
    setUnauthorizedHandler(null);
    vi.unstubAllGlobals();
  });

  it("sends the session cookie and surfaces the API error envelope", async () => {
    const handler = vi.fn();
    setUnauthorizedHandler(handler);
    const fetchMock = vi.fn(() =>
      Promise.resolve({
        ok: false,
        status: 401,
        json: () =>
          Promise.resolve({
            error: { code: "unauthorized", message: "missing session" },
          }),
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    await expect(apiRequest("/api/mcp-clients")).rejects.toMatchObject({
      message: "missing session",
      status: 401,
      code: "unauthorized",
    });

    expect(fetchMock).toHaveBeenCalledWith(
      "http://localhost:7010/api/mcp-clients",
      expect.objectContaining({ credentials: "include" }),
    );
    expect(handler).toHaveBeenCalledOnce();
  });

  it("posts JSON with credentials and does not read a 204 body", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce({
        ok: true,
        status: 204,
        json: () => Promise.reject(new Error("no body")),
      })
      .mockResolvedValueOnce({
        ok: true,
        status: 201,
        json: () => Promise.resolve({ id: "user-1", email: "a@example.com" }),
      });
    vi.stubGlobal("fetch", fetchMock);

    await expect(
      apiRequest("/api/auth/logout", { method: "POST", access: "public" }),
    ).resolves.toBeUndefined();

    await expect(
      apiRequest("/api/auth/register", {
        method: "POST",
        access: "public",
        body: { email: "a@example.com", password: "password1" },
      }),
    ).resolves.toEqual({ id: "user-1", email: "a@example.com" });

    expect(fetchMock).toHaveBeenLastCalledWith(
      "http://localhost:7010/api/auth/register",
      expect.objectContaining({
        method: "POST",
        credentials: "include",
        body: JSON.stringify({
          email: "a@example.com",
          password: "password1",
        }),
      }),
    );
  });
});
