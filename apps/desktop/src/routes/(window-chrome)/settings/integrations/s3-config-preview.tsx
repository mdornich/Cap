import { Button } from "@cap/ui-solid";
import { createSignal } from "solid-js";

interface S3Config {
  provider: string;
  accessKeyId: string;
  secretAccessKey: string;
  endpoint: string;
  bucketName: string;
  region: string;
}

const DEFAULT_CONFIG = {
  provider: "aws",
  accessKeyId: "",
  secretAccessKey: "",
  endpoint: "https://s3.amazonaws.com",
  bucketName: "",
  region: "us-east-1",
};

export default function S3ConfigPreview() {
  const [s3Config, setS3Config] = createSignal<S3Config>(DEFAULT_CONFIG);
  const [testStatus, setTestStatus] = createSignal("");
  const [saveStatus, setSaveStatus] = createSignal("");

  const renderInput = (
    label: string,
    key: keyof S3Config,
    placeholder: string,
    type: "text" | "password" = "text"
  ) => (
    <div>
      <label class="text-sm text-gray-12">{label}</label>
      <input
        type={type}
        value={s3Config()[key] ?? ""}
        onInput={(e: InputEvent & { currentTarget: HTMLInputElement }) =>
          setS3Config({ ...s3Config(), [key]: e.currentTarget.value })
        }
        placeholder={placeholder}
        class="px-3 py-2 w-full rounded-lg bg-gray-1 border border-gray-3 focus:outline-none focus:ring-2 focus:ring-blue-500"
        autocomplete="off"
        autocapitalize="off"
        autocorrect="off"
        spellcheck={false}
      />
    </div>
  );

  const handleTest = () => {
    setTestStatus("Testing connection...");
    setTimeout(() => {
      setTestStatus("✓ Connection test successful! (Preview mode)");
    }, 1500);
  };

  const handleSave = () => {
    setSaveStatus("Saving configuration...");
    setTimeout(() => {
      setSaveStatus("✓ Configuration saved! (Preview mode)");
    }, 1000);
  };

  return (
    <div class="flex flex-col h-full">
      {/* Preview Mode Banner */}
      <div class="bg-blue-500/20 border border-blue-500/50 p-2 text-center">
        <span class="text-sm text-blue-200">Preview Mode - No API calls are made</span>
      </div>
      
      <div class="overflow-y-auto flex-1 p-4">
        <div class="space-y-4 animate-in fade-in">
          <div>
            <p class="text-sm text-gray-11">
              It should take under 10 minutes to set up and connect your
              storage bucket to Cap. View the{" "}
              <a
                href="https://cap.so/docs/s3-config"
                target="_blank"
                class="font-semibold text-gray-12 underline"
              >
                Storage Config Guide
              </a>{" "}
              to get started.
            </p>
          </div>

          <div>
            <label class="text-sm text-gray-12">Storage Provider</label>
            <div class="relative">
              <select
                value={s3Config().provider}
                onChange={(e) =>
                  setS3Config((c) => ({
                    ...c,
                    provider: e.currentTarget.value,
                  }))
                }
                class="px-3 py-2 pr-10 w-full bg-gray-1 rounded-lg border border-gray-3 appearance-none focus:outline-none focus:ring-2 focus:ring-blue-500"
              >
                <option value="aws">AWS S3</option>
                <option value="cloudflare">Cloudflare R2</option>
                <option value="supabase">Supabase</option>
                <option value="minio">MinIO</option>
                <option value="other">Other S3-Compatible</option>
              </select>
              <div class="flex absolute inset-y-0 right-0 items-center px-2 pointer-events-none">
                <svg
                  class="w-4 h-4 text-gray-11"
                  xmlns="http://www.w3.org/2000/svg"
                  viewBox="0 0 20 20"
                  fill="currentColor"
                >
                  <path
                    fill-rule="evenodd"
                    d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                    clip-rule="evenodd"
                  />
                </svg>
              </div>
            </div>
          </div>

          {renderInput(
            "Access Key ID",
            "accessKeyId",
            "AKIAIOSFODNN7EXAMPLE",
            "password"
          )}
          {renderInput(
            "Secret Access Key",
            "secretAccessKey",
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            "password"
          )}
          {renderInput(
            "Endpoint",
            "endpoint",
            "https://s3.amazonaws.com"
          )}
          {renderInput("Bucket Name", "bucketName", "my-bucket")}
          {renderInput("Region", "region", "us-east-1")}

          {testStatus() && (
            <div class="p-3 bg-green-500/20 border border-green-500/50 rounded-lg">
              <span class="text-sm text-green-200">{testStatus()}</span>
            </div>
          )}

          {saveStatus() && (
            <div class="p-3 bg-green-500/20 border border-green-500/50 rounded-lg">
              <span class="text-sm text-green-200">{saveStatus()}</span>
            </div>
          )}
        </div>
      </div>

      <div class="flex-shrink-0 p-4 border-t">
        <div class="flex justify-between items-center">
          <div class="flex gap-2">
            <Button variant="secondary" onClick={handleTest}>
              Test Connection
            </Button>
          </div>
          <Button variant="primary" onClick={handleSave}>
            Save
          </Button>
        </div>
      </div>
    </div>
  );
}