import { Alert, Button, Chip, Paper, Stack, Typography } from "@mui/material";

import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router-dom";

import { errorText, mcpClientsApi, servicesApi } from "../../api";
import { useAuth } from "../../authContext";
import { queryKeys } from "../../queryKeys";
import { serviceKinds } from "../../serviceKinds";

function DashboardPage() {
  const { currentUser } = useAuth();
  const clientsQuery = useQuery({
    queryKey: queryKeys.mcpClients(),
    queryFn: mcpClientsApi.list,
  });
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
        Dashboard
      </Typography>
      <Typography color="text.secondary">
        Signed in as {currentUser?.email}
      </Typography>
      {clientsQuery.isError ? (
        <Alert severity="error">{errorText(clientsQuery.error)}</Alert>
      ) : null}
      {servicesQuery.isError ? (
        <Alert severity="error">{errorText(servicesQuery.error)}</Alert>
      ) : null}
      <Stack direction={{ xs: "column", sm: "row" }} spacing={2}>
        <Paper variant="outlined" sx={{ p: 2, flex: 1 }}>
          <Typography variant="overline">MCP clients</Typography>
          <Typography variant="h3">
            {clientsQuery.data?.clients.length ?? "—"}
          </Typography>
          <Button component={Link} to="/mcp-clients">
            Manage clients
          </Button>
        </Paper>
        <Paper variant="outlined" sx={{ p: 2, flex: 1 }}>
          <Typography variant="overline">Connected services</Typography>
          <Typography variant="h3">
            {servicesQuery.isSuccess ? connected.size : "—"}
          </Typography>
          <Button component={Link} to="/services">
            Manage services
          </Button>
        </Paper>
      </Stack>
      {servicesQuery.isSuccess ? (
        <Paper variant="outlined" sx={{ p: 2 }}>
          <Stack direction="row" spacing={1} useFlexGap flexWrap="wrap">
            {serviceKinds.map((service) => {
              const connection = connected.get(service.kind);
              return (
                <Chip
                  key={service.kind}
                  label={
                    connection
                      ? `${service.label}: ${connection.provider}`
                      : `${service.label}: not connected`
                  }
                  color={connection ? "primary" : "default"}
                  variant={connection ? "filled" : "outlined"}
                />
              );
            })}
          </Stack>
        </Paper>
      ) : null}
    </Stack>
  );
}

export default DashboardPage;
