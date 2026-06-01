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
  it("loads remote providers and current provider through one aggregated reader", async () => {
    const calls: string[] = [];

    const result = await loadProvidersQueryData("claude", remoteTarget, {
      getAll: async () => {
        calls.push("getAll");
        throw new Error("remote getAll should not be called");
      },
      getCurrent: async () => {
        calls.push("getCurrent");
        throw new Error("remote getCurrent should not be called");
      },
      getState: async () => {
        calls.push("getState");
        return {
          providers: {
            beta: {
              id: "beta",
              name: "Beta",
              settingsConfig: { env: {} },
              createdAt: 2,
            },
          },
          currentProviderId: "beta",
        };
      },
    });

    expect(calls).toEqual(["getState"]);
    expect(result.currentProviderId).toBe("beta");
    expect(Object.keys(result.providers)).toEqual(["beta"]);
  });

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
