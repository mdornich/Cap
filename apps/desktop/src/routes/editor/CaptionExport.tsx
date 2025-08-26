import { Button } from "@cap/ui-solid";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import { createSignal, Show, For, createEffect } from "solid-js";
import toast from "solid-toast";
import { commands } from "~/utils/tauri";
import { useEditorContext } from "./context";

interface CaptionExportFormat {
  name: string;
  extension: string;
  icon: string;
  description: string;
  exportFunction: (projectPath: string, outputPath?: string) => Promise<string>;
}

const CAPTION_FORMATS: CaptionExportFormat[] = [
  {
    name: "WebVTT",
    extension: "vtt",
    icon: "📄",
    description: "Web Video Text Tracks - For web players",
    exportFunction: async (projectPath: string, outputPath?: string) => {
      return await commands.exportCaptionsToVtt(projectPath, outputPath ?? null);
    },
  },
  {
    name: "SRT",
    extension: "srt",
    icon: "📝",
    description: "SubRip Text - Universal subtitle format",
    exportFunction: async (projectPath: string, outputPath?: string) => {
      return await commands.exportCaptionsToSrt(projectPath, outputPath ?? null);
    },
  },
  {
    name: "Plain Text",
    extension: "txt",
    icon: "📃",
    description: "Plain text transcript",
    exportFunction: async (projectPath: string, outputPath?: string) => {
      return await commands.exportCaptionsToText(projectPath, outputPath ?? null);
    },
  },
];

export function CaptionExport() {
  const { editorInstance, meta, project } = useEditorContext();
  const [hasCaptions, setHasCaptions] = createSignal(true); // Always show for now
  const [includeInVideo, setIncludeInVideo] = createSignal(false);
  const [isExporting, setIsExporting] = createSignal(false);
  const [exportingFormat, setExportingFormat] = createSignal<string | null>(null);

  const exportCaptions = async (format: CaptionExportFormat) => {
    if (isExporting()) return;

    try {
      setIsExporting(true);
      setExportingFormat(format.name);

      // Show save dialog
      const savePath = await saveDialog({
        filters: [
          {
            name: `${format.name} files`,
            extensions: [format.extension],
          },
        ],
        defaultPath: `${meta().prettyName}-captions.${format.extension}`,
      });

      if (!savePath) {
        setIsExporting(false);
        setExportingFormat(null);
        return;
      }

      // Export the captions
      await format.exportFunction(editorInstance.path, savePath);

      toast.success(`Captions exported as ${format.name}`);
    } catch (error) {
      console.error(`Failed to export captions as ${format.name}:`, error);
      toast.error(`Failed to export captions: ${error}`);
    } finally {
      setIsExporting(false);
      setExportingFormat(null);
    }
  };

  const downloadCaptionString = (content: string, filename: string) => {
    const blob = new Blob([content], { type: "text/plain;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  };

  const exportCaptionToString = async (format: CaptionExportFormat) => {
    if (isExporting()) return;

    try {
      setIsExporting(true);
      setExportingFormat(format.name);

      // Get caption content as string
      const content = await format.exportFunction(editorInstance.path);
      
      // Download the file
      const filename = `${meta().prettyName}-captions.${format.extension}`;
      downloadCaptionString(content, filename);

      toast.success(`Captions exported as ${format.name}`);
    } catch (error) {
      console.error(`Failed to export captions as ${format.name}:`, error);
      toast.error(`Failed to export captions: ${error}`);
    } finally {
      setIsExporting(false);
      setExportingFormat(null);
    }
  };

  return (
    <div class="p-4 rounded-xl bg-gray-2">
        <div class="flex flex-col gap-3">
          <div class="flex items-center justify-between">
            <h3 class="text-gray-12 flex items-center gap-2">
              <IconCapCaptions class="w-5 h-5" />
              Captions
            </h3>
            <Show when={project?.captions?.segments?.length}>
              <span class="text-xs text-gray-11">
                {project.captions.segments.length} segments
              </span>
            </Show>
          </div>

          {/* Include in video checkbox - for future burn-in feature */}
          <label class="flex items-center gap-2 cursor-pointer opacity-50" title="Burn-in captions (coming soon)">
            <input
              type="checkbox"
              checked={includeInVideo()}
              onChange={(e) => setIncludeInVideo(e.currentTarget.checked)}
              disabled
              class="w-4 h-4 rounded text-blue-500"
            />
            <span class="text-sm text-gray-11">Include captions in video (coming soon)</span>
          </label>

          {/* Export format buttons */}
          <div class="flex flex-col gap-2">
            <p class="text-xs text-gray-11">Export caption files separately:</p>
            <div class="grid grid-cols-3 gap-2">
              <For each={CAPTION_FORMATS}>
                {(format) => (
                  <Button
                    variant="secondary"
                    onClick={() => exportCaptionToString(format)}
                    disabled={isExporting()}
                    class="flex flex-col items-center gap-1 py-2 px-3 text-center"
                    title={format.description}
                  >
                    <Show
                      when={exportingFormat() === format.name}
                      fallback={
                        <>
                          <span class="text-lg">{format.icon}</span>
                          <span class="text-xs">{format.name}</span>
                        </>
                      }
                    >
                      <div class="flex items-center justify-center w-full h-full">
                        <IconLucideLoader2 class="animate-spin w-4 h-4" />
                      </div>
                    </Show>
                  </Button>
                )}
              </For>
            </div>
          </div>
        </div>
      </div>
  );
}

// Icons - these should be imported from your icon library
function IconCapCaptions(props: any) {
  return (
    <svg
      width="20"
      height="20"
      viewBox="0 0 20 20"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      {...props}
    >
      <path
        d="M2 5.5C2 4.67157 2.67157 4 3.5 4H16.5C17.3284 4 18 4.67157 18 5.5V14.5C18 15.3284 17.3284 16 16.5 16H3.5C2.67157 16 2 15.3284 2 14.5V5.5Z"
        stroke="currentColor"
        stroke-width="1.5"
      />
      <path
        d="M5 10H9"
        stroke="currentColor"
        stroke-width="1.5"
        stroke-linecap="round"
      />
      <path
        d="M5 12.5H12"
        stroke="currentColor"
        stroke-width="1.5"
        stroke-linecap="round"
      />
      <path
        d="M11 10H15"
        stroke="currentColor"
        stroke-width="1.5"
        stroke-linecap="round"
      />
    </svg>
  );
}

function IconLucideLoader2(props: any) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
      {...props}
    >
      <path d="M21 12a9 9 0 1 1-6.219-8.56" />
    </svg>
  );
}