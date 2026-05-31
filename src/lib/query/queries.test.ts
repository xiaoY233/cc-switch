import { describe, expect, it } from "vitest";
import { loadProvidersQueryData } from "@/lib/query/queries";
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

describe("provider query loading", () => {
  it("surfaces remote provider load errors instead of returning an empty list", async () => {
    const error = new Error("unsupported remote command");

    await expect(
      loadProvidersQueryData("claude", remoteTarget, {
        getAll: async () => {
          throw error;
        },
        getCurrent: async () => "anthropic",
      }),
    ).rejects.toThrow("unsupported remote command");
  });
});
