export const queryKeys = {
  auth: {
    me: () => ["auth", "me"] as const,
  },
  mcpClients: () => ["mcp-clients"] as const,
  services: () => ["services"] as const,
  actions: () => ["actions"] as const,
};
