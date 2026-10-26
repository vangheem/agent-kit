import { type FormEvent, useState } from "react";

import {
  Alert,
  Autocomplete,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Paper,
  Stack,
  TextField,
  Typography,
} from "@mui/material";

import { mdiContentCopy } from "@mdi/js";
import Icon from "@mdi/react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { actionsApi, errorText, mcpClientsApi } from "../../api";
import { queryKeys } from "../../queryKeys";
import type { McpClient } from "../../types";

function formatWhen(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  return date.toLocaleString();
}

function McpClientsPage() {
  const queryClient = useQueryClient();
  const clientsQuery = useQuery({
    queryKey: queryKeys.mcpClients(),
    queryFn: mcpClientsApi.list,
  });
  const actionsQuery = useQuery({
    queryKey: queryKeys.actions(),
    queryFn: actionsApi.list,
  });
  const [name, setName] = useState("");
  const [enabledActions, setEnabledActions] = useState<string[]>([]);
  const [issuedToken, setIssuedToken] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [pendingRevoke, setPendingRevoke] = useState<McpClient | null>(null);

  const createClient = useMutation({
    mutationFn: mcpClientsApi.create,
    onSuccess: async (created) => {
      setIssuedToken(created.token);
      setCopied(false);
      setName("");
      setEnabledActions([]);
      await queryClient.invalidateQueries({ queryKey: queryKeys.mcpClients() });
    },
  });
  const revokeClient = useMutation({
    mutationFn: mcpClientsApi.revoke,
    onSuccess: async () => {
      setPendingRevoke(null);
      await queryClient.invalidateQueries({ queryKey: queryKeys.mcpClients() });
    },
  });

  function onCreate(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const trimmed = name.trim();
    if (trimmed.length === 0) {
      return;
    }
    createClient.mutate({ name: trimmed, enabled_actions: enabledActions });
  }

  async function copyToken() {
    if (!issuedToken) {
      return;
    }
    try {
      await navigator.clipboard.writeText(issuedToken);
      setCopied(true);
    } catch {
      setCopied(false);
    }
  }

  const actionOptions = (actionsQuery.data?.actions ?? []).map(
    (action) => action.id,
  );

  return (
    <Stack spacing={2}>
      <Typography variant="h5" component="h2">
        MCP clients
      </Typography>
      <Typography color="text.secondary">
        Agents authenticate with a bearer token. The raw token is shown once.
      </Typography>
      {issuedToken ? (
        <Alert
          severity="warning"
          action={
            <Button
              color="inherit"
              size="small"
              onClick={() => void copyToken()}
              startIcon={<Icon path={mdiContentCopy} size={0.7} />}
            >
              {copied ? "Copied" : "Copy"}
            </Button>
          }
        >
          <Stack spacing={1}>
            <Typography variant="body2">
              Copy this token now. It will not be shown again.
            </Typography>
            <TextField
              value={issuedToken}
              fullWidth
              size="small"
              slotProps={{ input: { readOnly: true } }}
            />
            <Button size="small" onClick={() => setIssuedToken(null)}>
              Dismiss
            </Button>
          </Stack>
        </Alert>
      ) : null}
      <Paper variant="outlined" sx={{ p: 2 }}>
        <Stack spacing={2} component="form" onSubmit={onCreate}>
          <TextField
            label="Name"
            value={name}
            onChange={(event) => setName(event.target.value)}
            required
            fullWidth
          />
          <Autocomplete
            multiple
            freeSolo
            options={actionOptions}
            value={enabledActions}
            onChange={(_event, value) => setEnabledActions(value.map(String))}
            renderInput={(params) => (
              <TextField
                {...params}
                label="Enabled actions"
                helperText="Leave empty to allow every action."
              />
            )}
          />
          {createClient.isError ? (
            <Alert severity="error">{errorText(createClient.error)}</Alert>
          ) : null}
          <Button
            type="submit"
            variant="contained"
            disabled={createClient.isPending}
          >
            Create client
          </Button>
        </Stack>
      </Paper>
      {clientsQuery.isError ? (
        <Alert severity="error">{errorText(clientsQuery.error)}</Alert>
      ) : null}
      {clientsQuery.isSuccess && clientsQuery.data.clients.length === 0 ? (
        <Typography color="text.secondary">No clients yet.</Typography>
      ) : null}
      {(clientsQuery.data?.clients ?? []).map((client) => (
        <Paper key={client.id} variant="outlined" sx={{ p: 2 }}>
          <Stack
            direction={{ xs: "column", sm: "row" }}
            spacing={1}
            justifyContent="space-between"
          >
            <Stack spacing={0.5}>
              <Typography fontWeight={600}>{client.name}</Typography>
              <Typography variant="body2" color="text.secondary">
                Prefix {client.token_prefix} · {formatWhen(client.created_at)}
              </Typography>
              <Typography variant="body2">
                {client.enabled_actions.length === 0
                  ? "All actions enabled"
                  : client.enabled_actions.join(", ")}
              </Typography>
            </Stack>
            <Button
              color="error"
              onClick={() => setPendingRevoke(client)}
              disabled={revokeClient.isPending}
            >
              Revoke
            </Button>
          </Stack>
        </Paper>
      ))}
      <Dialog
        open={pendingRevoke !== null}
        onClose={() => setPendingRevoke(null)}
      >
        <DialogTitle>Revoke client</DialogTitle>
        <DialogContent>
          Revoke {pendingRevoke?.name}? Agents using its token will be rejected.
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setPendingRevoke(null)}>Cancel</Button>
          <Button
            color="error"
            onClick={() => {
              if (pendingRevoke) {
                revokeClient.mutate(pendingRevoke.id);
              }
            }}
          >
            Revoke
          </Button>
        </DialogActions>
      </Dialog>
    </Stack>
  );
}

export default McpClientsPage;
