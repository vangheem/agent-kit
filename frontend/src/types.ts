export type User = {
  id: string;
  email: string;
};

export type McpClient = {
  id: string;
  name: string;
  token_prefix: string;
  enabled_actions: string[];
  created_at: string;
};

export type CreatedMcpClient = McpClient & {
  token: string;
};

export type ServiceConnection = {
  kind: string;
  provider: string;
  configured: boolean;
};

export type ActionDescriptor = {
  id: string;
  title: string;
  summary: string;
  input_schema: unknown;
  output_schema: unknown;
  requires: string[];
  effect: string;
};
