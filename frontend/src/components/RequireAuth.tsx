import type { ReactNode } from "react";

import { Alert, LinearProgress } from "@mui/material";

import { Navigate, useLocation } from "react-router-dom";

import { useAuth } from "../authContext";

export function RequireAuth({ children }: { children: ReactNode }) {
  const { currentUser, isAuthReady, authError } = useAuth();
  const location = useLocation();

  if (!isAuthReady) {
    return <LinearProgress />;
  }

  if (authError) {
    return <Alert severity="error">{authError.message}</Alert>;
  }

  if (!currentUser) {
    return <Navigate to="/login" replace state={{ from: location.pathname }} />;
  }

  return children;
}
