import { describe, expect, it } from "vitest";
import { remoteQueryKeys } from "./remote";

describe("remote query keys", () => {
  it("scopes session status by host id", () => {
    expect(remoteQueryKeys.session("host-1")).toEqual([
      "remote",
      "host",
      "host-1",
      "session",
    ]);
  });
});
