import { type ReactElement, createElement } from "react";

import { RequireAuth } from "./components/RequireAuth";
import ActionsPage from "./pages/actions";
import DashboardPage from "./pages/dashboard";
import LoginPage from "./pages/login";
import McpClientsPage from "./pages/mcp-clients";
import RegisterPage from "./pages/register";
import ServicesPage from "./pages/services";

export type AppRoute = {
  path: string;
  element: ReactElement;
};

function protectedPage(page: () => ReactElement): ReactElement {
  return createElement(RequireAuth, null, createElement(page));
}

export const routes: AppRoute[] = [
  {
    path: "/login",
    element: createElement(LoginPage),
  },
  {
    path: "/register",
    element: createElement(RegisterPage),
  },
  {
    path: "/",
    element: protectedPage(DashboardPage),
  },
  {
    path: "/mcp-clients",
    element: protectedPage(McpClientsPage),
  },
  {
    path: "/services",
    element: protectedPage(ServicesPage),
  },
  {
    path: "/actions",
    element: protectedPage(ActionsPage),
  },
];
