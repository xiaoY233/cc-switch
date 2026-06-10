import { describe, expect, it } from "vitest";
import {
  getApiKeyFromConfig,
  hasRedactedApiKeyValue,
  hideRemoteProviderConfigSecretsForDisplay,
  isCodexRemoteCompactionEnabled,
  restoreRemoteProviderConfigSecretsForSubmit,
  setCodexRemoteCompaction,
} from "./providerConfigUtils";

describe("provider secret redaction helpers", () => {
  it("does not expose redacted Claude API keys as editable input values", () => {
    const config = JSON.stringify({
      env: {
        ANTHROPIC_AUTH_TOKEN: "[redacted]",
        ANTHROPIC_BASE_URL: "https://api.example.com",
      },
    });

    expect(getApiKeyFromConfig(config, "claude")).toBe("");
    expect(hasRedactedApiKeyValue(config, "claude")).toBe(true);
  });

  it("detects redacted top-level API keys without treating them as display values", () => {
    const config = JSON.stringify({
      apiKey: "[redacted]",
      baseUrl: "https://api.example.com",
    });

    expect(getApiKeyFromConfig(config, "openclaw")).toBe("");
    expect(hasRedactedApiKeyValue(config, "openclaw")).toBe(true);
  });

  it("still returns real API keys for local non-redacted configs", () => {
    const config = JSON.stringify({
      env: {
        GEMINI_API_KEY: "real-key",
      },
    });

    expect(getApiKeyFromConfig(config, "gemini")).toBe("real-key");
    expect(hasRedactedApiKeyValue(config, "gemini")).toBe(false);
  });

  it("hides redacted values from remote JSON editors but restores placeholders on submit", () => {
    const original = {
      env: {
        ANTHROPIC_AUTH_TOKEN: "[redacted]",
        ANTHROPIC_BASE_URL: "https://api.example.com",
      },
    };

    const display = hideRemoteProviderConfigSecretsForDisplay(
      "claude",
      original,
    );

    expect(display.env).toEqual({
      ANTHROPIC_AUTH_TOKEN: "",
      ANTHROPIC_BASE_URL: "https://api.example.com",
    });
    expect(JSON.stringify(display)).not.toContain("[redacted]");

    const restored = restoreRemoteProviderConfigSecretsForSubmit(
      "claude",
      original,
      display,
    );
    expect(restored).toEqual(original);
  });

  it("keeps a newly entered remote secret instead of restoring the placeholder", () => {
    const original = {
      apiKey: "[redacted]",
      baseUrl: "https://api.example.com",
    };
    const incoming = {
      apiKey: "new-key",
      baseUrl: "https://api.example.com",
    };

    expect(
      restoreRemoteProviderConfigSecretsForSubmit(
        "openclaw",
        original,
        incoming,
      ),
    ).toEqual(incoming);
  });

  it("hides and restores redacted Codex auth and experimental bearer token", () => {
    const original = {
      auth: {
        OPENAI_API_KEY: "[redacted]",
      },
      config: `model_provider = "custom"

[model_providers.custom]
base_url = "https://api.example.com/v1"
experimental_bearer_token = "[redacted]"
`,
    };

    const display = hideRemoteProviderConfigSecretsForDisplay(
      "codex",
      original,
    );

    expect(JSON.stringify(display)).not.toContain("[redacted]");
    expect(display.config).toContain('experimental_bearer_token = ""');

    const restored = restoreRemoteProviderConfigSecretsForSubmit(
      "codex",
      original,
      display,
    );
    expect(restored).toEqual(original);
  });
});

describe("Codex remote compaction config helpers", () => {
  it("enables remote compaction by naming the active custom provider OpenAI", () => {
    const input = `model_provider = "custom"
model = "gpt-5.4"

[model_providers.custom]
name = "AIHubMix"
base_url = "https://aihubmix.example/v1"
wire_api = "responses"

[model_providers.backup]
name = "Backup"
base_url = "https://backup.example/v1"
`;

    const result = setCodexRemoteCompaction(input, true, "AIHubMix");

    expect(isCodexRemoteCompactionEnabled(result)).toBe(true);
    expect(result).toContain(`[model_providers.custom]\nname = "OpenAI"`);
    expect(result).toContain(`[model_providers.backup]\nname = "Backup"`);
  });

  it("disables remote compaction by restoring the provider display name", () => {
    const input = `model_provider = "custom"

[model_providers.custom]
name = "OpenAI"
base_url = "https://aihubmix.example/v1"
wire_api = "responses"
`;

    const result = setCodexRemoteCompaction(input, false, "AIHubMix");

    expect(isCodexRemoteCompactionEnabled(result)).toBe(false);
    expect(result).toContain(`name = "AIHubMix"`);
  });

  it("does not rewrite reserved built-in providers", () => {
    const input = `model_provider = "openai"
model = "gpt-5"
`;

    expect(setCodexRemoteCompaction(input, true, "OpenAI")).toBe(input);
    expect(isCodexRemoteCompactionEnabled(input)).toBe(false);
  });
});
