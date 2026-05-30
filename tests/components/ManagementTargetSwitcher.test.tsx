import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ManagementTargetSwitcher } from "@/components/remote/ManagementTargetSwitcher";
import type { RemoteHostProfile } from "@/lib/api";

const profiles: RemoteHostProfile[] = [
  {
    id: "remote-1",
    name: "生产服务器",
    host: "192.168.123.203",
    port: 22,
    username: "root",
    authMethod: { type: "password" },
    helperPath: "~/.local/bin/cc-switch-remote-helper",
    createdAt: 1,
    updatedAt: 1,
  },
];

describe("ManagementTargetSwitcher", () => {
  it("uses the project segmented-switcher style for local and remote targets", () => {
    render(
      <ManagementTargetSwitcher
        profiles={profiles}
        activeTargetKey="local"
        onTargetChange={vi.fn()}
      />,
    );

    const switcher = screen.getByLabelText("管理目标");
    expect(switcher).toHaveClass("inline-flex", "rounded-xl", "bg-muted");
    expect(screen.getByRole("button", { name: "本地" })).toHaveClass(
      "bg-background",
      "shadow-sm",
    );
    expect(screen.getByRole("button", { name: /远程/ })).not.toHaveClass(
      "bg-background",
    );
  });

  it("selects a remote profile from the styled target menu", async () => {
    const onTargetChange = vi.fn();
    const user = userEvent.setup();
    render(
      <ManagementTargetSwitcher
        profiles={profiles}
        activeTargetKey="local"
        onTargetChange={onTargetChange}
      />,
    );

    await user.click(screen.getByRole("button", { name: /远程/ }));
    await user.click(await screen.findByText("生产服务器"));

    expect(onTargetChange).toHaveBeenCalledWith("remote:remote-1");
  });

  it("shows the active remote name in the switcher", () => {
    render(
      <ManagementTargetSwitcher
        profiles={profiles}
        activeTargetKey="remote:remote-1"
        onTargetChange={vi.fn()}
      />,
    );

    expect(screen.getByRole("button", { name: /生产服务器/ })).toHaveClass(
      "bg-background",
      "shadow-sm",
    );
  });
});
