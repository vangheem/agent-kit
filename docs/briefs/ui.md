# ui

Read `docs/PLAN.md` and `docs/CONTRACTS.md`. Also look at `/Users/vangheem/projects/personal/acuitysync/frontend` for structure only: page modules, `routes.ts`, thin `App.tsx`, `src/api`, TanStack Query, MUI. Do not copy AcuitySync branding, copy, or business screens.

## Own

Everything under `frontend/`, including `Dockerfile`, `nginx.conf` if you need one, and `Makefile` with `install`, `run`, `build`, `test`, `lint`, `format`.

## npm

Commit `frontend/.npmrc` with the public registry:

```
registry=https://registry.npmjs.org/
```

Dockerfile `NPM_REGISTRY` defaults to that URL. `npm install` / `npm ci` only.

Dev server port `4210`, host `0.0.0.0`. `VITE_API_URL` default `http://localhost:7010`.

## Screens

Login, register, dashboard, MCP clients, services, actions. Cookie session. Show a newly created MCP token once. Service forms send `{ provider, credentials }` and never expect secrets back.

One useful Vitest, not a static copy assertion. Do not commit. Do not start the root compose file.
