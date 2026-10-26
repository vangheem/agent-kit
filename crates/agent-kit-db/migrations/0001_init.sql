CREATE EXTENSION IF NOT EXISTS citext;
CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE users (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  email citext NOT NULL UNIQUE,
  password_hash text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE sessions (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  token_hash text NOT NULL UNIQUE,
  expires_at timestamptz NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE mcp_clients (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  name text NOT NULL,
  token_hash text NOT NULL UNIQUE,
  token_prefix text NOT NULL,
  enabled_actions text[] NOT NULL DEFAULT '{}',
  created_at timestamptz NOT NULL DEFAULT now(),
  revoked_at timestamptz
);

CREATE TABLE service_connections (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  kind text NOT NULL,
  provider text NOT NULL,
  credentials_ciphertext bytea NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now(),
  UNIQUE (user_id, kind)
);

CREATE TABLE blobs (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  owner_user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  content_type text NOT NULL,
  byte_size bigint NOT NULL,
  storage_backend text NOT NULL,
  storage_key text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE blob_payloads (
  blob_id uuid PRIMARY KEY REFERENCES blobs(id) ON DELETE CASCADE,
  data bytea NOT NULL
);

CREATE TABLE action_runs (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  mcp_client_id uuid NOT NULL REFERENCES mcp_clients(id) ON DELETE CASCADE,
  action_id text NOT NULL,
  status text NOT NULL,
  input_blob_id uuid REFERENCES blobs(id),
  output_blob_id uuid REFERENCES blobs(id),
  error_code text,
  duration_ms integer,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX sessions_user_id_idx ON sessions (user_id);
CREATE INDEX mcp_clients_user_id_idx ON mcp_clients (user_id);
CREATE INDEX service_connections_user_id_idx ON service_connections (user_id);
CREATE INDEX blobs_owner_user_id_idx ON blobs (owner_user_id);
CREATE INDEX action_runs_mcp_client_id_idx ON action_runs (mcp_client_id);
