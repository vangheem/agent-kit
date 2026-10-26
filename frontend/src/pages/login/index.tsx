import { type FormEvent, useState } from "react";

import {
  Alert,
  Box,
  Button,
  LinearProgress,
  Paper,
  Stack,
  TextField,
  Typography,
} from "@mui/material";

import {
  Navigate,
  Link as RouterLink,
  useLocation,
  useNavigate,
} from "react-router-dom";

import { errorText } from "../../api";
import { useAuth } from "../../authContext";

function LoginPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const { currentUser, isAuthReady, login } = useAuth();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [loginError, setLoginError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const fromState = (location.state as { from?: string } | null)?.from;
  const from =
    fromState &&
    fromState.startsWith("/") &&
    !fromState.startsWith("//") &&
    fromState !== "/login"
      ? fromState
      : "/";

  if (!isAuthReady) {
    return <LinearProgress />;
  }

  if (currentUser) {
    return <Navigate to="/" replace />;
  }

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setBusy(true);
    setLoginError(null);
    try {
      await login(email, password);
      setPassword("");
      navigate(from);
    } catch (error) {
      setLoginError(errorText(error));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Paper variant="outlined" sx={{ p: 3, maxWidth: 440 }}>
      <Stack spacing={2} component="form" onSubmit={onSubmit}>
        <Typography variant="h5" component="h2">
          Login
        </Typography>
        {loginError ? <Alert severity="error">{loginError}</Alert> : null}
        <TextField
          label="Email"
          type="email"
          value={email}
          onChange={(event) => setEmail(event.target.value)}
          autoComplete="username"
          required
          fullWidth
        />
        <TextField
          label="Password"
          type="password"
          value={password}
          onChange={(event) => setPassword(event.target.value)}
          autoComplete="current-password"
          required
          fullWidth
        />
        <Button type="submit" variant="contained" disabled={busy}>
          Login
        </Button>
        <Box>
          <Button component={RouterLink} to="/register">
            Create account
          </Button>
        </Box>
      </Stack>
    </Paper>
  );
}

export default LoginPage;
