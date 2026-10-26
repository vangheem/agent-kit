export const serviceKinds = [
  { kind: "web_search", label: "Web search" },
  { kind: "shop", label: "Shop" },
  { kind: "flights", label: "Flights" },
  { kind: "markets", label: "Markets" },
] as const;

export type ServiceKind = (typeof serviceKinds)[number]["kind"];
