import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { FailoverQueueManager } from "@/components/proxy/FailoverQueueManager";
import type { ManagementTarget, RemoteHostProfile } from "@/lib/api";

const getFailoverQueueMock = vi.fn();
const getAvailableProvidersForFailoverMock = vi.fn();
const addToFailoverQueueMock = vi.fn();
const removeFromFailoverQueueMock = vi.fn();
const getAutoFailoverEnabledMock = vi.fn();
const setAutoFailoverEnabledMock = vi.fn();

vi.mock("@/lib/api/failover", () => ({
  failoverApi: {
    getFailoverQueue: (...args: unknown[]) => getFailoverQueueMock(...args),
    getAvailableProvidersForFailover: (...args: unknown[]) =>
      getAvailableProvidersForFailoverMock(...args),
    addToFailoverQueue: (...args: unknown[]) => addToFailoverQueueMock(...args),
    removeFromFailoverQueue: (...args: unknown[]) =>
      removeFromFailoverQueueMock(...args),
    getAutoFailoverEnabled: (...args: unknown[]) =>
      getAutoFailoverEnabledMock(...args),
    setAutoFailoverEnabled: (...args: unknown[]) =>
      setAutoFailoverEnabledMock(...args),
  },
}));

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

const profile: RemoteHostProfile = {
  id: "remote-1",
  name: "Swarm01",
  host: "192.168.123.203",
  port: 22,
  username: "root",
  authMethod: { type: "password" },
  helperPath: "~/.local/bin/cc-switch-remote-helper",
  createdAt: 1,
  updatedAt: 1,
};

const remoteTarget: ManagementTarget = {
  type: "remote",
  profile,
  secret: { password: "secret" },
};

function renderWithQueryClient(ui: React.ReactNode) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>,
  );
}

describe("FailoverQueueManager", () => {
  beforeEach(() => {
    getFailoverQueueMock.mockReset();
    getAvailableProvidersForFailoverMock.mockReset();
    addToFailoverQueueMock.mockReset();
    removeFromFailoverQueueMock.mockReset();
    getAutoFailoverEnabledMock.mockReset();
    setAutoFailoverEnabledMock.mockReset();

    getFailoverQueueMock.mockResolvedValue([]);
    getAvailableProvidersForFailoverMock.mockResolvedValue([
      {
        id: "remote-provider",
        name: "Remote Provider",
        settingsConfig: {},
      },
    ]);
    addToFailoverQueueMock.mockResolvedValue(undefined);
    removeFromFailoverQueueMock.mockResolvedValue(undefined);
    getAutoFailoverEnabledMock.mockResolvedValue(false);
    setAutoFailoverEnabledMock.mockResolvedValue(undefined);
  });

  it("allows remote queue edits while automatic failover is disabled by routing preconditions", async () => {
    const user = userEvent.setup();

    renderWithQueryClient(
      <FailoverQueueManager
        appType="codex"
        target={remoteTarget}
        autoSwitchDisabled
      />,
    );

    await user.click(await screen.findByRole("combobox"));
    await user.click(screen.getByRole("option", { name: "Remote Provider" }));
    await user.click(
      screen.getByRole("button", { name: "添加到故障转移队列" }),
    );

    await waitFor(() => {
      expect(addToFailoverQueueMock).toHaveBeenCalledWith(
        "codex",
        "remote-provider",
        remoteTarget,
      );
    });

    expect(screen.getByRole("switch", { name: "自动故障转移" })).toBeDisabled();
  });
});
