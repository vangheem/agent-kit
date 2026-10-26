import { createContext, useContext } from "react";

import type { User } from "./types";

export type AuthContextValue = {
  currentUser: User | null;
  isAuthReady: boolean;
  authError: Error | null;
  login: (email: string, password: string) => Promise<User>;
  register: (email: string, password: string) => Promise<User>;
  logout: () => Promise<void>;
};

export const AuthContext = createContext<AuthContextValue | null>(null);

export function useAuth() {
  const context = useContext(AuthContext);
  if (context === null) {
    throw new Error("useAuth must be used within AuthProvider");
  }
  return context;
}
