import { describe, expect, it } from "vitest";
import type { RemoteHealth } from "./api";
import {
  formatRemoteHelperLatest,
  formatRemoteHelperVersion,
} from "./remoteHealth";

function health(overrides: Partial<RemoteHealth> = {}): RemoteHealth {
  return {
    reachable: true,
    helperInstalled: true,
    capabilities: [],
    ...overrides,
  };
}

describe("remote helper version formatting", () => {
  it("includes the helper build so same-version helper updates are visible", () => {
    const remoteHealth = health({
      helperVersion: "3.16.4",
      helperBuild: "9f1869de1234567890",
      helperLatestVersion: "3.16.4",
      helperLatestBuild: "b98d8acb1234567890",
    });

    expect(formatRemoteHelperVersion(remoteHealth)).toBe("3.16.4 (9f1869de)");
    expect(formatRemoteHelperLatest(remoteHealth)).toBe("3.16.4 (b98d8acb)");
  });

  it("keeps legacy helper versions readable when no build is reported", () => {
    const remoteHealth = health({
      helperVersion: "3.16.4",
      helperLatestVersion: "3.16.4",
    });

    expect(formatRemoteHelperVersion(remoteHealth)).toBe("3.16.4");
    expect(formatRemoteHelperLatest(remoteHealth)).toBe("3.16.4");
  });
});
