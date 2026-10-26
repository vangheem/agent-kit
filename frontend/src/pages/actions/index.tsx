import {
  Accordion,
  AccordionDetails,
  AccordionSummary,
  Alert,
  Chip,
  Stack,
  Typography,
} from "@mui/material";

import { mdiChevronDown } from "@mdi/js";
import Icon from "@mdi/react";
import { useQuery } from "@tanstack/react-query";

import { actionsApi, errorText } from "../../api";
import { queryKeys } from "../../queryKeys";

function effectColor(effect: string): "warning" | "default" {
  return effect.toLowerCase().includes("mutat") ? "warning" : "default";
}

function ActionsPage() {
  const actionsQuery = useQuery({
    queryKey: queryKeys.actions(),
    queryFn: actionsApi.list,
  });

  return (
    <Stack spacing={2}>
      <Typography variant="h5" component="h2">
        Actions
      </Typography>
      <Typography color="text.secondary">
        Read-only catalog from the API. Agents discover these through MCP
        search.
      </Typography>
      {actionsQuery.isError ? (
        <Alert severity="error">{errorText(actionsQuery.error)}</Alert>
      ) : null}
      {(actionsQuery.data?.actions ?? []).map((action) => (
        <Accordion key={action.id} variant="outlined" disableGutters>
          <AccordionSummary
            expandIcon={<Icon path={mdiChevronDown} size={0.9} />}
          >
            <Stack spacing={0.5} sx={{ py: 1 }}>
              <Stack direction="row" spacing={1} useFlexGap flexWrap="wrap">
                <Typography fontWeight={700}>{action.id}</Typography>
                <Chip
                  size="small"
                  label={action.effect}
                  color={effectColor(action.effect)}
                />
                {action.requires.map((kind) => (
                  <Chip
                    key={kind}
                    size="small"
                    variant="outlined"
                    label={kind}
                  />
                ))}
              </Stack>
              <Typography>{action.title}</Typography>
              <Typography variant="body2" color="text.secondary">
                {action.summary}
              </Typography>
            </Stack>
          </AccordionSummary>
          <AccordionDetails>
            <Stack spacing={1}>
              <Typography variant="overline">Input schema</Typography>
              <SchemaBlock value={action.input_schema} />
              <Typography variant="overline">Output schema</Typography>
              <SchemaBlock value={action.output_schema} />
            </Stack>
          </AccordionDetails>
        </Accordion>
      ))}
    </Stack>
  );
}

function SchemaBlock({ value }: { value: unknown }) {
  return (
    <Typography
      component="pre"
      variant="body2"
      sx={{
        m: 0,
        p: 1.5,
        overflow: "auto",
        maxHeight: 240,
        bgcolor: "action.hover",
        borderRadius: 1,
        fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
      }}
    >
      {JSON.stringify(value, null, 2)}
    </Typography>
  );
}

export default ActionsPage;
