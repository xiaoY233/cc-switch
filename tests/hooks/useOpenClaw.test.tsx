import type { ReactNode } from "react";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useOpenClawDefaultModel } from "@/hooks/useOpenClaw";
import type { ManagementTarget } from "@/lib/api";

const openclawApiGetDefaultModelMock = vi.fn();
const remoteApiGetOpenClawDefaultModelMock = vi.fn();

vi.mock("@/lib/api/openclaw", () => ({
  openclawApi: {
    getDefaultModel: (...args: unknown[]) =>
      openclawApiGetDefaultModelMock(...args),
  },
}));

vi.mock("@/lib/api/remote", () => ({
  remoteApi: {
    getOpenClawDefaultModel: (...args: unknown[]) =>
      remoteApiGetOpenClawDefaultModelMock(...args),
  },
}));

interface WrapperProps {
  children: ReactNode;
}

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const wrapper = ({ children }: WrapperProps) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
  return { wrapper };
}

describe("useOpenClawDefaultModel", () => {
  beforeEach(() => {
    openclawApiGetDefaultModelMock.mockReset();
    remoteApiGetOpenClawDefaultModelMock.mockReset();
  });

  it("queries the remote helper for remote targets", async () => {
    remoteApiGetOpenClawDefaultModelMock.mockResolvedValueOnce({
      primary: "remote-provider/gpt-4.1",
    });
    const remoteTarget: ManagementTarget = {
      type: "remote",
      profile: {
        id: "remote-1",
        name: "Remote 1",
        host: "192.168.1.20",
        port: 22,
        username: "root",
        authMethod: { type: "password" },
        helperPath: "~/.local/bin/cc-switch-remote-helper",
        createdAt: 1,
        updatedAt: 1,
      },
      secret: { password: "secret" },
    };
    const { wrapper } = createWrapper();

    const { result } = renderHook(
      () => useOpenClawDefaultModel(true, remoteTarget),
      { wrapper },
    );

    await waitFor(() => {
      expect(result.current.data?.primary).toBe("remote-provider/gpt-4.1");
    });

    expect(remoteApiGetOpenClawDefaultModelMock).toHaveBeenCalledWith(
      remoteTarget.profile,
      remoteTarget.secret,
    );
    expect(openclawApiGetDefaultModelMock).not.toHaveBeenCalled();
  });
});
