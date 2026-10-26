import { type FormEvent, useState } from "react";

import {
  Alert,
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

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { errorText, servicesApi } from "../../api";
import { parseCredentials } from "../../api/services";
import { queryKeys } from "../../queryKeys";
import { type ServiceKind, serviceKinds } from "../../serviceKinds";
import type { ServiceConnection } from "../../types";

function ServicesPage() {
  const servicesQuery = useQuery({
    queryKey: queryKeys.services(),
    queryFn: servicesApi.list,
  });
  const connected = new Map(
    (servicesQuery.data?.services ?? [])
      .filter((service) => service.configured)
      .map((service) => [service.kind, service]),
  );

  return (
    <Stack spacing={2}>
      <Typography variant="h5" component="h2">
        Services
      </Typography>
      <Typography color="text.secondary">
        Credentials are stored encrypted and are not returned by the API.
      </Typography>
      {servicesQuery.isError ? (
        <Alert severity="error">{errorText(servicesQuery.error)}</Alert>
      ) : null}
      {serviceKinds.map((service) => (
        <ServiceCard
          key={`${service.kind}:${connected.get(service.kind)?.provider ?? "new"}`}
          kind={service.kind}
          label={service.label}
          connection={connected.get(service.kind)}
        />
      ))}
    </Stack>
  );
}

function ServiceCard({
  kind,
  label,
  connection,
}: {
  kind: ServiceKind;
  label: string;
  connection?: ServiceConnection;
}) {
  const queryClient = useQueryClient();
  const [provider, setProvider] = useState(connection?.provider ?? "mock");
  const [credentials, setCredentials] = useState("{}");
  const [formError, setFormError] = useState<string | null>(null);
  const [confirmRemove, setConfirmRemove] = useState(false);

  const save = useMutation({
    mutationFn: (body: {
      provider: string;
      credentials: Record<string, unknown>;
    }) => servicesApi.save(kind, body),
    onSuccess: async () => {
      setCredentials("{}");
      setFormError(null);
      await queryClient.invalidateQueries({ queryKey: queryKeys.services() });
    },
  });
  const remove = useMutation({
    mutationFn: () => servicesApi.remove(kind),
    onSuccess: async () => {
      setConfirmRemove(false);
      setProvider("mock");
      await queryClient.invalidateQueries({ queryKey: queryKeys.services() });
    },
  });

  function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const trimmed = provider.trim();
    if (trimmed.length === 0) {
      setFormError("Provider is required");
      return;
    }
    try {
      save.mutate({
        provider: trimmed,
        credentials: parseCredentials(credentials),
      });
      setFormError(null);
    } catch (error) {
      setFormError(errorText(error));
    }
  }

  return (
    <Paper variant="outlined" sx={{ p: 2 }}>
      <Stack spacing={2} component="form" onSubmit={onSubmit}>
        <Stack
          direction="row"
          justifyContent="space-between"
          alignItems="center"
        >
          <Typography fontWeight={600}>{label}</Typography>
          <Typography variant="body2" color="text.secondary">
            {connection
              ? `Connected as ${connection.provider}`
              : "Not connected"}
          </Typography>
        </Stack>
        <TextField
          label="Provider"
          value={provider}
          onChange={(event) => setProvider(event.target.value)}
          helperText="mock is the local adapter."
          required
          fullWidth
        />
        <TextField
          label="Credentials"
          value={credentials}
          onChange={(event) => setCredentials(event.target.value)}
          helperText="JSON object. Stored secrets are not shown."
          multiline
          minRows={3}
          fullWidth
          spellCheck={false}
        />
        {formError ? <Alert severity="error">{formError}</Alert> : null}
        {save.isError ? (
          <Alert severity="error">{errorText(save.error)}</Alert>
        ) : null}
        {remove.isError ? (
          <Alert severity="error">{errorText(remove.error)}</Alert>
        ) : null}
        <Stack direction="row" spacing={1}>
          <Button type="submit" variant="contained" disabled={save.isPending}>
            {connection ? "Update" : "Connect"}
          </Button>
          {connection ? (
            <Button
              color="error"
              onClick={() => setConfirmRemove(true)}
              disabled={remove.isPending}
            >
              Remove
            </Button>
          ) : null}
        </Stack>
      </Stack>
      <Dialog open={confirmRemove} onClose={() => setConfirmRemove(false)}>
        <DialogTitle>Remove {label}</DialogTitle>
        <DialogContent>
          This deletes the stored credentials. They cannot be displayed again.
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setConfirmRemove(false)}>Cancel</Button>
          <Button color="error" onClick={() => remove.mutate()}>
            Remove
          </Button>
        </DialogActions>
      </Dialog>
    </Paper>
  );
}

export default ServicesPage;
