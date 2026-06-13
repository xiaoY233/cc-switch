import { act, renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  useAddToFailoverQueue,
  useCircuitBreakerConfig,
  useProviderHealth,
  useResetCircuitBreaker,
  useSetAutoFailoverEnabled,
  useUpdateCircuitBreakerConfig,
} from "@/lib/query/failover";
import type { ManagementTarget, RemoteHostProfile } from "@/lib/api";

const setAutoFailoverEnabledMock = vi.fn();
const addToFailoverQueueMock = vi.fn();
const getProviderHealthMock = vi.fn();
const resetCircuitBreakerMock = vi.fn();
const getCircuitBreakerConfigMock = vi.fn();
const updateCircuitBreakerConfigMock = vi.fn();

vi.mock("@/lib/api/failover", () => ({
  failoverApi: {
    setAutoFailoverEnabled: (...args: unknown[]) =>
      setAutoFailoverEnabledMock(...args),
    addToFailoverQueue: (...args: unknown[]) => addToFailoverQueueMock(...args),
    getProviderHealth: (...args: unknown[]) => getProviderHealthMock(...args),
    resetCircuitBreaker: (...args: unknown[]) =>
      resetCircuitBreakerMock(...args),
    getCircuitBreakerConfig: (...args: unknown[]) =>
      getCircuitBreakerConfigMock(...args),
    updateCircuitBreakerConfig: (...args: unknown[]) =>
      updateCircuitBreakerConfigMock(...args),
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

function createWrapper(queryClient: QueryClient) {
  return function Wrapper({ children }: { children: React.ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );
  };
}

describe("failover target query keys", () => {
  beforeEach(() => {
    setAutoFailoverEnabledMock.mockReset();
    addToFailoverQueueMock.mockReset();
    getProviderHealthMock.mockReset();
    resetCircuitBreakerMock.mockReset();
    getCircuitBreakerConfigMock.mockReset();
    updateCircuitBreakerConfigMock.mockReset();
    setAutoFailoverEnabledMock.mockResolvedValue(undefined);
    addToFailoverQueueMock.mockResolvedValue(undefined);
    getProviderHealthMock.mockResolvedValue({ isHealthy: true });
    resetCircuitBreakerMock.mockResolvedValue(undefined);
    getCircuitBreakerConfigMock.mockResolvedValue({
      failureThreshold: 4,
      successThreshold: 2,
      timeoutSeconds: 60,
      errorRateThreshold: 0.6,
      minRequests: 10,
    });
    updateCircuitBreakerConfigMock.mockResolvedValue(undefined);
  });

  it("invalidates only target-keyed caches after remote auto failover changes", async () => {
    const queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
        mutations: { retry: false },
      },
    });
    const invalidateSpy = vi.spyOn(queryClient, "invalidateQueries");

    const { result } = renderHook(
      () => useSetAutoFailoverEnabled(remoteTarget),
      { wrapper: createWrapper(queryClient) },
    );

    await act(async () => {
      await result.current.mutateAsync({ appType: "codex", enabled: true });
    });

    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: ["autoFailoverEnabled", "remote:remote-1", "codex"],
    });
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: ["failoverQueue", "remote:remote-1", "codex"],
    });
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: ["availableProvidersForFailover", "remote:remote-1", "codex"],
    });
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: ["providers", "codex", "remote:remote-1"],
    });
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: ["proxyStatus", "remote:remote-1"],
    });
    expect(invalidateSpy).not.toHaveBeenCalledWith({
      queryKey: ["providers", "codex"],
    });
    expect(invalidateSpy).not.toHaveBeenCalledWith({
      queryKey: ["proxyStatus"],
    });
  });

  it("invalidates target-keyed provider caches after adding a remote failover provider", async () => {
    const queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
        mutations: { retry: false },
      },
    });
    const invalidateSpy = vi.spyOn(queryClient, "invalidateQueries");

    const { result } = renderHook(() => useAddToFailoverQueue(remoteTarget), {
      wrapper: createWrapper(queryClient),
    });

    await act(async () => {
      await result.current.mutateAsync({
        appType: "claude",
        providerId: "p1",
      });
    });

    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: ["failoverQueue", "remote:remote-1", "claude"],
    });
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: ["availableProvidersForFailover", "remote:remote-1", "claude"],
    });
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: ["providers", "claude", "remote:remote-1"],
    });
    expect(invalidateSpy).not.toHaveBeenCalledWith({
      queryKey: ["providers", "claude"],
    });
  });

  it("uses target-keyed caches for remote provider health", async () => {
    const queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
        mutations: { retry: false },
      },
    });

    const { result } = renderHook(
      () => useProviderHealth("remote-provider", "codex", remoteTarget),
      { wrapper: createWrapper(queryClient) },
    );

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(getProviderHealthMock).toHaveBeenCalledWith(
      "remote-provider",
      "codex",
      remoteTarget,
    );
    expect(
      queryClient.getQueryData([
        "providerHealth",
        "remote:remote-1",
        "remote-provider",
        "codex",
      ]),
    ).toEqual({ isHealthy: true });
    expect(
      queryClient.getQueryData(["providerHealth", "remote-provider", "codex"]),
    ).toBeUndefined();
  });

  it("invalidates only target-keyed circuit caches after remote circuit reset", async () => {
    const queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
        mutations: { retry: false },
      },
    });
    const invalidateSpy = vi.spyOn(queryClient, "invalidateQueries");

    const { result } = renderHook(() => useResetCircuitBreaker(remoteTarget), {
      wrapper: createWrapper(queryClient),
    });

    await act(async () => {
      await result.current.mutateAsync({
        appType: "codex",
        providerId: "remote-provider",
      });
    });

    expect(resetCircuitBreakerMock).toHaveBeenCalledWith(
      "remote-provider",
      "codex",
      remoteTarget,
    );
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: [
        "providerHealth",
        "remote:remote-1",
        "remote-provider",
        "codex",
      ],
    });
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: [
        "circuitBreakerStats",
        "remote:remote-1",
        "remote-provider",
        "codex",
      ],
    });
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: ["providers", "codex", "remote:remote-1"],
    });
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: ["proxyStatus", "remote:remote-1"],
    });
    expect(invalidateSpy).not.toHaveBeenCalledWith({
      queryKey: ["providers", "codex"],
    });
    expect(invalidateSpy).not.toHaveBeenCalledWith({
      queryKey: ["proxyStatus"],
    });
  });

  it("uses target-keyed circuit breaker config queries and mutations", async () => {
    const queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
        mutations: { retry: false },
      },
    });
    const invalidateSpy = vi.spyOn(queryClient, "invalidateQueries");

    const { result: configResult } = renderHook(
      () => useCircuitBreakerConfig(remoteTarget),
      { wrapper: createWrapper(queryClient) },
    );

    await waitFor(() => {
      expect(configResult.current.isSuccess).toBe(true);
    });

    expect(getCircuitBreakerConfigMock).toHaveBeenCalledWith(remoteTarget);
    expect(
      queryClient.getQueryData(["circuitBreakerConfig", "remote:remote-1"]),
    ).toEqual({
      failureThreshold: 4,
      successThreshold: 2,
      timeoutSeconds: 60,
      errorRateThreshold: 0.6,
      minRequests: 10,
    });
    expect(queryClient.getQueryData(["circuitBreakerConfig"])).toBeUndefined();

    const { result: mutationResult } = renderHook(
      () => useUpdateCircuitBreakerConfig(remoteTarget),
      { wrapper: createWrapper(queryClient) },
    );

    await act(async () => {
      await mutationResult.current.mutateAsync({
        failureThreshold: 5,
        successThreshold: 2,
        timeoutSeconds: 90,
        errorRateThreshold: 0.5,
        minRequests: 10,
      });
    });

    expect(updateCircuitBreakerConfigMock).toHaveBeenCalledWith(
      {
        failureThreshold: 5,
        successThreshold: 2,
        timeoutSeconds: 90,
        errorRateThreshold: 0.5,
        minRequests: 10,
      },
      remoteTarget,
    );
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: ["circuitBreakerConfig", "remote:remote-1"],
    });
    expect(invalidateSpy).not.toHaveBeenCalledWith({
      queryKey: ["circuitBreakerConfig"],
    });
  });
});
