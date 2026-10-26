import { type ReactNode, useEffect } from "react";

import { useQuery, useQueryClient } from "@tanstack/react-query";

import { ApiError, authApi } from "./api";
import { setUnauthorizedHandler } from "./api/client";
import { AuthContext } from "./authContext";
import { queryKeys } from "./queryKeys";
import type { User } from "./types";

function clearSession(queryClient: ReturnType<typeof useQueryClient>): void {
  queryClient.setQueryData<User | null>(queryKeys.auth.me(), null);
  queryClient.removeQueries({
    predicate: (query) => query.queryKey[0] !== "auth",
  });
}

export function AuthProvider({ children }: { children: ReactNode }) {
  const queryClient = useQueryClient();
  const meQuery = useQuery({
    queryKey: queryKeys.auth.me(),
    queryFn: ({ signal }) => authApi.me({ signal }),
    retry: false,
  });

  useEffect(() => {
    setUnauthorizedHandler(() => {
      clearSession(queryClient);
    });
    return () => setUnauthorizedHandler(null);
  }, [queryClient]);

  const sessionMissing =
    meQuery.error instanceof ApiError && meQuery.error.status === 401;
  const currentUser = meQuery.data ?? null;
  const isAuthReady = meQuery.isSuccess || meQuery.isError;
  const authError =
    meQuery.error instanceof Error && !sessionMissing ? meQuery.error : null;

  async function login(email: string, password: string) {
    const user = await authApi.login({ email, password });
    queryClient.setQueryData<User>(queryKeys.auth.me(), user);
    return user;
  }

  async function register(email: string, password: string) {
    await authApi.register({ email, password });
    return login(email, password);
  }

  async function logout() {
    try {
      await authApi.logout();
    } finally {
      clearSession(queryClient);
    }
  }

  return (
    <AuthContext.Provider
      value={{
        currentUser,
        isAuthReady,
        authError,
        login,
        register,
        logout,
      }}
    >
      {children}
    </AuthContext.Provider>
  );
}
