import { describe, expect, it } from "vitest";
import {
  getManagementTargetKey,
  shouldReadLocalLiveConfig,
} from "@/lib/managementTarget";
import type { ManagementTarget } from "@/lib/api";

const remoteTarget: ManagementTarget = {
  type: "remote",
  profile: {
    id: "server-1",
    name: "Server 1",
    host: "192.168.1.10",
    port: 22,
    username: "root",
    authMethod: { type: "password" },
    helperPath: "~/.local/bin/cc-switch-remote-helper",
    createdAt: 1,
    updatedAt: 1,
  },
  secret: { password: "secret" },
};

describe("management target helpers", () => {
  it("uses stable cache keys for local and remote targets", () => {
    expect(getManagementTargetKey({ type: "local" })).toBe("local");
    expect(getManagementTargetKey(remoteTarget)).toBe("remote:server-1");
  });

  it("does not read local live config for remote targets", () => {
    expect(shouldReadLocalLiveConfig({ type: "local" }, false)).toBe(true);
    expect(shouldReadLocalLiveConfig({ type: "local" }, true)).toBe(false);
    expect(shouldReadLocalLiveConfig(remoteTarget, false)).toBe(false);
  });
});
