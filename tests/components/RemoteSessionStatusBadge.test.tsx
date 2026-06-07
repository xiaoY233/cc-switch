import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { RemoteSessionStatusBadge } from "@/components/remote/RemoteSessionStatusBadge";

describe("RemoteSessionStatusBadge", () => {
  it("uses a low-noise failed state instead of a destructive red background", () => {
    render(
      <RemoteSessionStatusBadge
        status={{
          profileId: "remote-1",
          state: "failed",
          lastError: "connection failed",
        }}
      />,
    );

    const badge = screen.getByTestId("remote-session-status");
    expect(badge).toHaveClass("bg-background");
    expect(badge).not.toHaveClass("bg-destructive");
  });
});
