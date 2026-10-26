import {
  Box,
  Button,
  Chip,
  Container,
  CssBaseline,
  LinearProgress,
  Stack,
  ThemeProvider,
  Typography,
  createTheme,
} from "@mui/material";

import {
  mdiConnection,
  mdiFormatListBulleted,
  mdiKeyVariant,
  mdiLogin,
  mdiLogout,
  mdiViewDashboard,
} from "@mdi/js";
import Icon from "@mdi/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  BrowserRouter,
  Link,
  Navigate,
  Route,
  Routes,
  useNavigate,
} from "react-router-dom";

import { AuthProvider } from "./auth";
import { useAuth } from "./authContext";
import { routes } from "./routes";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: false,
      refetchOnWindowFocus: false,
    },
  },
});

const theme = createTheme({
  palette: {
    mode: "light",
    primary: {
      main: "#0f3d5e",
    },
    secondary: {
      main: "#5c6b73",
    },
    background: {
      default: "#eef2f5",
      paper: "#ffffff",
    },
  },
  shape: {
    borderRadius: 8,
  },
  typography: {
    fontFamily: "Roboto, Arial, sans-serif",
  },
  components: {
    MuiButton: {
      styleOverrides: {
        root: {
          textTransform: "none",
        },
      },
    },
  },
});

const navItems = [
  { to: "/", label: "Dashboard", icon: mdiViewDashboard },
  { to: "/mcp-clients", label: "MCP clients", icon: mdiKeyVariant },
  { to: "/services", label: "Services", icon: mdiConnection },
  { to: "/actions", label: "Actions", icon: mdiFormatListBulleted },
];

function App() {
  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <QueryClientProvider client={queryClient}>
        <BrowserRouter>
          <AuthProvider>
            <AppShell />
          </AuthProvider>
        </BrowserRouter>
      </QueryClientProvider>
    </ThemeProvider>
  );
}

function AppShell() {
  const { currentUser, isAuthReady, logout } = useAuth();
  const navigate = useNavigate();

  async function onLogout() {
    await logout();
    navigate("/login");
  }

  return (
    <Box component="main" sx={{ minHeight: "100vh", py: 3 }}>
      <Container maxWidth="lg">
        <Stack spacing={3}>
          <Stack
            direction={{ xs: "column", md: "row" }}
            spacing={2}
            justifyContent="space-between"
            alignItems={{ xs: "flex-start", md: "center" }}
            sx={{
              px: 2,
              py: 1.5,
              border: 1,
              borderColor: "divider",
              borderRadius: 2,
              bgcolor: "background.paper",
            }}
          >
            <Box>
              <Typography variant="h5" component="h1" fontWeight={700}>
                <Link
                  to="/"
                  style={{ color: "inherit", textDecoration: "none" }}
                >
                  agent-kit
                </Link>
              </Typography>
              <Typography variant="body2" color="text.secondary">
                Search and execute, without driving a browser.
              </Typography>
            </Box>
            {!isAuthReady ? (
              <LinearProgress sx={{ width: 160 }} />
            ) : currentUser ? (
              <Stack direction="row" spacing={1} useFlexGap flexWrap="wrap">
                {navItems.map((item) => (
                  <Button
                    key={item.to}
                    component={Link}
                    to={item.to}
                    startIcon={<Icon path={item.icon} size={0.75} />}
                  >
                    {item.label}
                  </Button>
                ))}
                <Chip label={currentUser.email} />
                <Button
                  variant="outlined"
                  onClick={() => void onLogout()}
                  startIcon={<Icon path={mdiLogout} size={0.75} />}
                >
                  Logout
                </Button>
              </Stack>
            ) : (
              <Stack direction="row" spacing={1}>
                <Button
                  component={Link}
                  to="/login"
                  variant="contained"
                  startIcon={<Icon path={mdiLogin} size={0.75} />}
                >
                  Login
                </Button>
                <Button component={Link} to="/register" variant="outlined">
                  Register
                </Button>
              </Stack>
            )}
          </Stack>
          <Routes>
            {routes.map((route) => (
              <Route
                key={route.path}
                path={route.path}
                element={route.element}
              />
            ))}
            <Route path="*" element={<RouteFallback />} />
          </Routes>
        </Stack>
      </Container>
    </Box>
  );
}

function RouteFallback() {
  const { currentUser, isAuthReady } = useAuth();
  if (!isAuthReady) {
    return <LinearProgress />;
  }
  return <Navigate to={currentUser ? "/" : "/login"} replace />;
}

export default App;
