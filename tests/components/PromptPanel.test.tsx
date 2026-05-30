import { createRef } from "react";
import { render, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import PromptPanel, {
  type PromptPanelHandle,
} from "@/components/prompts/PromptPanel";
import type { ManagementTarget } from "@/lib/api";

const usePromptActionsMock = vi.fn();
const reloadMock = vi.fn();
const importFromFileMock = vi.fn();

vi.mock("@/hooks/usePromptActions", () => ({
  usePromptActions: (...args: unknown[]) => usePromptActionsMock(...args),
}));

vi.mock("@/components/prompts/PromptListItem", () => ({
  default: () => <div data-testid="prompt-list-item" />,
}));

vi.mock("@/components/prompts/PromptFormPanel", () => ({
  default: () => <div data-testid="prompt-form-panel" />,
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

beforeEach(() => {
  usePromptActionsMock.mockReset();
  reloadMock.mockReset();
  importFromFileMock.mockReset();
  usePromptActionsMock.mockReturnValue({
    prompts: {},
    loading: false,
    reload: reloadMock,
    savePrompt: vi.fn(),
    deletePrompt: vi.fn(),
    toggleEnabled: vi.fn(),
    importFromFile: importFromFileMock,
  });
});

describe("PromptPanel", () => {
  it("exposes import action for remote targets", async () => {
    const ref = createRef<PromptPanelHandle>();
    importFromFileMock.mockResolvedValueOnce("remote-prompt");

    render(
      <PromptPanel
        ref={ref}
        open
        appId="claude"
        target={remoteTarget}
        onOpenChange={vi.fn()}
      />,
    );

    await waitFor(() => expect(ref.current).not.toBeNull());
    expect(usePromptActionsMock).toHaveBeenCalledWith("claude", remoteTarget);

    await ref.current!.openImport();

    expect(importFromFileMock).toHaveBeenCalledTimes(1);
  });
});
