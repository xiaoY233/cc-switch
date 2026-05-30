import type { ReactNode } from "react";
import { renderHook, act, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { usePromptActions } from "@/hooks/usePromptActions";
import type { ManagementTarget } from "@/lib/api/remote";
import type { Prompt } from "@/lib/api";

const toastSuccessMock = vi.hoisted(() => vi.fn());
const toastErrorMock = vi.hoisted(() => vi.fn());
const getPromptsMock = vi.hoisted(() => vi.fn());
const upsertPromptMock = vi.hoisted(() => vi.fn());
const deletePromptMock = vi.hoisted(() => vi.fn());
const enablePromptMock = vi.hoisted(() => vi.fn());
const importFromFileMock = vi.hoisted(() => vi.fn());
const getCurrentFileContentMock = vi.hoisted(() => vi.fn());

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock("@/lib/api", () => ({
  promptsApi: {
    getPrompts: (...args: unknown[]) => getPromptsMock(...args),
    upsertPrompt: (...args: unknown[]) => upsertPromptMock(...args),
    deletePrompt: (...args: unknown[]) => deletePromptMock(...args),
    enablePrompt: (...args: unknown[]) => enablePromptMock(...args),
    importFromFile: (...args: unknown[]) => importFromFileMock(...args),
    getCurrentFileContent: (...args: unknown[]) =>
      getCurrentFileContentMock(...args),
  },
}));

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

const prompt: Prompt = {
  id: "default",
  name: "Default",
  content: "Remote prompt",
  enabled: true,
};

interface WrapperProps {
  children: ReactNode;
}

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  const wrapper = ({ children }: WrapperProps) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );

  return { wrapper };
}

function mockReload(prompts: Record<string, Prompt> = { default: prompt }) {
  getPromptsMock.mockResolvedValue(prompts);
  getCurrentFileContentMock.mockResolvedValue("remote file content");
}

beforeEach(() => {
  toastSuccessMock.mockReset();
  toastErrorMock.mockReset();
  getPromptsMock.mockReset();
  upsertPromptMock.mockReset();
  deletePromptMock.mockReset();
  enablePromptMock.mockReset();
  importFromFileMock.mockReset();
  getCurrentFileContentMock.mockReset();
});

describe("usePromptActions remote target", () => {
  it("loads prompts and current prompt file content from the selected remote target", async () => {
    mockReload();
    const { wrapper } = createWrapper();
    const { result } = renderHook(
      () => usePromptActions("claude", remoteTarget),
      { wrapper },
    );

    await act(async () => {
      await result.current.reload();
    });

    await waitFor(() =>
      expect(result.current.prompts).toEqual({ default: prompt }),
    );
    expect(result.current.currentFileContent).toBe("remote file content");
    expect(getPromptsMock).toHaveBeenCalledWith("claude", remoteTarget);
    expect(getCurrentFileContentMock).toHaveBeenCalledWith(
      "claude",
      remoteTarget,
    );
  });

  it("saves, deletes, and enables prompts through the selected remote target", async () => {
    mockReload();
    upsertPromptMock.mockResolvedValue(undefined);
    deletePromptMock.mockResolvedValue(undefined);
    enablePromptMock.mockResolvedValue(undefined);
    const { wrapper } = createWrapper();
    const { result } = renderHook(
      () => usePromptActions("codex", remoteTarget),
      { wrapper },
    );

    await act(async () => {
      await result.current.savePrompt("default", prompt);
      await result.current.deletePrompt("default");
      await result.current.enablePrompt("default");
    });

    expect(upsertPromptMock).toHaveBeenCalledWith(
      "codex",
      "default",
      prompt,
      remoteTarget,
    );
    expect(deletePromptMock).toHaveBeenCalledWith(
      "codex",
      "default",
      remoteTarget,
    );
    expect(enablePromptMock).toHaveBeenCalledWith(
      "codex",
      "default",
      remoteTarget,
    );
  });

  it("disables a prompt by upserting the remote prompt with enabled false", async () => {
    mockReload({ default: prompt });
    upsertPromptMock.mockResolvedValue(undefined);
    const { wrapper } = createWrapper();
    const { result } = renderHook(
      () => usePromptActions("gemini", remoteTarget),
      { wrapper },
    );

    await act(async () => {
      await result.current.reload();
    });
    await act(async () => {
      await result.current.toggleEnabled("default", false);
    });

    expect(upsertPromptMock).toHaveBeenCalledWith(
      "gemini",
      "default",
      { ...prompt, enabled: false },
      remoteTarget,
    );
  });

  it("imports prompts from the selected remote target", async () => {
    mockReload();
    importFromFileMock.mockResolvedValueOnce("imported-1");
    const { wrapper } = createWrapper();
    const { result } = renderHook(
      () => usePromptActions("openclaw", remoteTarget),
      { wrapper },
    );

    let importedId: string | undefined;
    await act(async () => {
      importedId = await result.current.importFromFile();
    });

    expect(importedId).toBe("imported-1");
    expect(importFromFileMock).toHaveBeenCalledWith("openclaw", remoteTarget);
  });
});
