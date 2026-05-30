import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { RemoteServersPage } from "@/components/remote/RemoteServersPage";
import type { RemoteHostProfile } from "@/lib/api";

const saveProfileMock = vi.fn();
const listProfilesMock = vi.fn();
const deleteProfileMock = vi.fn();

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<any>("@/lib/api");
  return {
    ...actual,
    remoteApi: {
      ...actual.remoteApi,
      saveProfile: (...args: unknown[]) => saveProfileMock(...args),
      listProfiles: (...args: unknown[]) => listProfilesMock(...args),
      deleteProfile: (...args: unknown[]) => deleteProfileMock(...args),
    },
  };
});

describe("RemoteServersPage", () => {
  beforeEach(() => {
    saveProfileMock.mockReset();
    listProfilesMock.mockReset();
    deleteProfileMock.mockReset();
  });

  it("saves a password-auth remote profile and keeps the password as a session secret", async () => {
    const user = userEvent.setup();
    const onProfileSaved = vi.fn();
    const onProfileActivated = vi.fn();
    const onProfilesChanged = vi.fn();
    saveProfileMock.mockImplementation(async (profile: RemoteHostProfile) => ({
      ...profile,
      id: "remote-saved",
    }));
    listProfilesMock.mockResolvedValueOnce([]);

    render(
      <RemoteServersPage
        profiles={[]}
        onProfileSaved={onProfileSaved}
        onProfileActivated={onProfileActivated}
        onProfilesChanged={onProfilesChanged}
      />,
    );

    await user.click(screen.getAllByRole("button", { name: "新增服务器" })[0]);
    await user.click(screen.getByRole("button", { name: "密码" }));

    const textboxes = screen.getAllByRole("textbox");
    await user.type(textboxes[0], "测试服务器");
    await user.type(screen.getByPlaceholderText("10.0.0.10"), "192.168.123.203");
    await user.type(screen.getByPlaceholderText("deploy"), "root");
    await user.clear(textboxes[3]);
    await user.type(textboxes[3], "22");
    await user.type(screen.getByLabelText("密码"), "session-password");
    await user.click(screen.getByRole("button", { name: "保存" }));

    expect(saveProfileMock).toHaveBeenCalledWith(
      expect.objectContaining({
        name: "测试服务器",
        host: "192.168.123.203",
        username: "root",
        port: 22,
        authMethod: { type: "password" },
      }),
    );
    expect(onProfileSaved).toHaveBeenCalledWith(
      expect.objectContaining({
        id: "remote-saved",
        authMethod: { type: "password" },
      }),
      { password: "session-password" },
    );
    expect(onProfilesChanged).toHaveBeenCalledWith([]);
    expect(onProfileActivated).toHaveBeenCalledWith("remote-saved");
  });
});
