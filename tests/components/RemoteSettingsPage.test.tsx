import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { RemoteSettingsPage } from "@/components/settings/RemoteSettingsPage";
import type { RemoteHostProfile } from "@/lib/api";
import type { Settings } from "@/types";

const checkHealthMock = vi.fn();
const getSettingsMock = vi.fn();
const saveSettingsMock = vi.fn();
const getInstalledSkillsMock = vi.fn();
const migrateSkillStorageMock = vi.fn();

function createDeferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<any>("@/lib/api");
  return {
    ...actual,
    remoteApi: {
      ...actual.remoteApi,
      checkHealth: (...args: unknown[]) => checkHealthMock(...args),
      getSettings: (...args: unknown[]) => getSettingsMock(...args),
      saveSettings: (...args: unknown[]) => saveSettingsMock(...args),
      getInstalledSkills: (...args: unknown[]) =>
        getInstalledSkillsMock(...args),
      migrateSkillStorage: (...args: unknown[]) =>
        migrateSkillStorageMock(...args),
    },
  };
});

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

const settings: Settings = {
  showInTray: true,
  minimizeToTrayOnClose: true,
  useAppWindowControls: false,
  enableClaudePluginIntegration: false,
  skipClaudeOnboarding: false,
  launchOnStartup: false,
  silentStartup: false,
  enableLocalProxy: false,
  visibleApps: {
    claude: true,
    "claude-desktop": true,
    codex: true,
    gemini: true,
    opencode: true,
    openclaw: true,
    hermes: true,
  },
  skillSyncMethod: "auto",
  skillStorageLocation: "cc_switch",
};

describe("RemoteSettingsPage", () => {
  beforeEach(() => {
    checkHealthMock.mockReset();
    getSettingsMock.mockReset();
    saveSettingsMock.mockReset();
    getInstalledSkillsMock.mockReset();
    migrateSkillStorageMock.mockReset();

    checkHealthMock.mockResolvedValue({
      reachable: true,
      helperInstalled: true,
      helperVersion: "3.16.2",
      platform: "linux",
      capabilities: ["settings", "plugin", "skills", "tools"],
    });
    getSettingsMock.mockResolvedValue(settings);
    saveSettingsMock.mockResolvedValue(true);
    getInstalledSkillsMock.mockResolvedValue([]);
  });

  it("loads remote general settings and saves per-host app visibility", async () => {
    const user = userEvent.setup();

    render(
      <RemoteSettingsPage
        open
        onOpenChange={vi.fn()}
        target={{ type: "remote", profile, secret: { password: "secret" } }}
      />,
    );

    expect(screen.getByRole("tab", { name: "远程" })).toHaveAttribute(
      "aria-selected",
      "true",
    );

    await user.click(screen.getByRole("tab", { name: "通用" }));

    await waitFor(() => {
      expect(screen.getByText("远程通用设置")).toBeInTheDocument();
    });

    await user.click(screen.getByRole("button", { name: /gemini/i }));

    await waitFor(() => {
      expect(saveSettingsMock).toHaveBeenCalledWith(
        profile,
        expect.objectContaining({
          visibleApps: expect.objectContaining({ gemini: false }),
        }),
        { password: "secret" },
      );
    });
  });

  it("keeps the selected remote settings tab after async health refresh", async () => {
    const user = userEvent.setup();
    const health =
      createDeferred<Awaited<ReturnType<typeof checkHealthMock>>>();
    checkHealthMock.mockReturnValueOnce(health.promise);

    render(
      <RemoteSettingsPage
        open
        onOpenChange={vi.fn()}
        defaultTab="general"
        target={{ type: "remote", profile, secret: { password: "secret" } }}
      />,
    );

    await user.click(screen.getByRole("tab", { name: "远程" }));
    expect(screen.getByRole("tab", { name: "远程" })).toHaveAttribute(
      "aria-selected",
      "true",
    );

    health.resolve({
      reachable: true,
      helperInstalled: true,
      helperVersion: "3.16.2",
      platform: "linux",
      capabilities: ["settings", "plugin", "skills", "tools"],
    });

    await waitFor(() => {
      expect(getSettingsMock).toHaveBeenCalled();
    });

    expect(screen.getByRole("tab", { name: "远程" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(screen.getByRole("tab", { name: "通用" })).toHaveAttribute(
      "aria-selected",
      "false",
    );
  });
});
