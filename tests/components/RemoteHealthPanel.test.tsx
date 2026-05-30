import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import { RemoteHealthPanel } from "@/components/remote/RemoteHealthPanel";
import type { RemoteHostProfile } from "@/lib/api";

const checkHealthMock = vi.fn();
const installHelperMock = vi.fn();

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<any>("@/lib/api");
  return {
    ...actual,
    remoteApi: {
      ...actual.remoteApi,
      checkHealth: (...args: unknown[]) => checkHealthMock(...args),
      installHelper: (...args: unknown[]) => installHelperMock(...args),
    },
  };
});

const profile: RemoteHostProfile = {
  id: "remote-1",
  name: "Remote 1",
  host: "192.168.123.203",
  port: 22,
  username: "root",
  authMethod: { type: "password" },
  helperPath: "/root/cc-switch-current/src-tauri/target/debug/cc-switch-cli",
  createdAt: 1,
  updatedAt: 1,
};

describe("RemoteHealthPanel", () => {
  beforeEach(() => {
    checkHealthMock.mockReset();
    installHelperMock.mockReset();
  });

  it("warns when the remote helper is missing a parity capability", async () => {
    checkHealthMock.mockResolvedValueOnce({
      reachable: true,
      helperInstalled: true,
      helperVersion: "3.16.0",
      platform: "linux",
      capabilities: ["providers", "mcp", "prompts", "skills", "import-export"],
    });

    render(
      <RemoteHealthPanel
        profile={profile}
        secret={{ password: "session-secret" }}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "检查" }));

    await waitFor(() => {
      expect(screen.getByText("缺少能力")).toBeInTheDocument();
    });
    expect(screen.getAllByText("OpenClaw").length).toBeGreaterThan(0);
    expect(screen.getByText("供应商")).toBeInTheDocument();
    expect(checkHealthMock).toHaveBeenCalledWith(profile, {
      password: "session-secret",
    });
  });

  it("shows all helper capabilities without a missing-capability warning", async () => {
    checkHealthMock.mockResolvedValueOnce({
      reachable: true,
      helperInstalled: true,
      helperVersion: "3.16.0",
      platform: "linux",
      capabilities: [
        "providers",
        "openclaw",
        "mcp",
        "prompts",
        "skills",
        "import-export",
      ],
    });

    render(<RemoteHealthPanel profile={profile} />);

    fireEvent.click(screen.getByRole("button", { name: "检查" }));

    await waitFor(() => {
      expect(screen.getByText("OpenClaw")).toBeInTheDocument();
    });
    expect(screen.queryByText("缺少能力")).not.toBeInTheDocument();
  });

  it("keeps the configured helper path visible after a health check", async () => {
    checkHealthMock.mockResolvedValueOnce({
      reachable: true,
      helperInstalled: true,
      helperVersion: "3.16.0",
      platform: "linux",
      capabilities: ["providers"],
    });

    render(<RemoteHealthPanel profile={profile} />);

    expect(screen.getByText(profile.helperPath)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "检查" }));

    await waitFor(() => {
      expect(screen.getByText("3.16.0")).toBeInTheDocument();
    });
    expect(screen.getByText(profile.helperPath)).toBeInTheDocument();
  });
});
