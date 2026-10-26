import { type FormEvent, useState } from "react";

import {
  Alert,
  Button,
  LinearProgress,
  Paper,
  Stack,
  TextField,
  Typography,
} from "@mui/material";

import { Navigate, Link as RouterLink, useNavigate } from "react-router-dom";

import { errorText } from "../../api";
import { useAuth } from "../../authContext";

function RegisterPage() {
  const navigate = useNavigate();
  const { currentUser, isAuthReady, register } = useAuth();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [formError, setFormError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  if (!isAuthReady) {
    return <LinearProgress />;
  }

  if (currentUser) {
    return <Navigate to="/" replace />;
  }

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!email.includes("@")) {
      setFormError("Email must contain @");
      return;
    }
    if (password.length < 8) {
      setFormError("Password must be at least 8 characters");
      return;
    }
    setBusy(true);
    setFormError(null);
    try {
      await register(email, password);
      setPassword("");
      navigate("/");
    } catch (error) {
      setFormError(errorText(error));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Paper variant="outlined" sx={{ p: 3, maxWidth: 440 }}>
      <Stack spacing={2} component="form" onSubmit={onSubmit}>
        <Typography variant="h5" component="h2">
          Create account
        </Typography>
        {formError ? <Alert severity="error">{formError}</Alert> : null}
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
          autoComplete="new-password"
          required
          slotProps={{ htmlInput: { minLength: 8 } }}
          fullWidth
        />
        <Button type="submit" variant="contained" disabled={busy}>
          Register
        </Button>
        <Button component={RouterLink} to="/login">
          Back to login
        </Button>
      </Stack>
    </Paper>
  );
}

export default RegisterPage;
