import { createElementBounds } from "@solid-primitives/bounds";
import { createEventListener } from "@solid-primitives/event-listener";
import { For, Show, batch, createRoot, createSignal } from "solid-js";
import { produce } from "solid-js/store";

import { useEditorContext } from "../context";
import { useTimelineContext } from "./context";
import type { SceneMode, SceneSegment } from "~/utils/tauri";

export type SceneSegmentDragState =
  | { type: "idle" }
  | { type: "moving"; index: number; startMouseX: number; startSegment: SceneSegment }
  | { type: "resizing-start"; index: number; startMouseX: number; startSegment: SceneSegment }
  | { type: "resizing-end"; index: number; startMouseX: number; startSegment: SceneSegment };

export function SceneTrack(props: {
  onDragStateChanged?: (state: SceneSegmentDragState) => void;
  handleUpdatePlayhead: (e: MouseEvent) => void;
}) {
  const { project, setProject, editorState, setEditorState } = useEditorContext();
  const { secsPerPixel, timelineBounds } = useTimelineContext();

  const transform = () => editorState.timeline.transform;
  const selection = () => editorState.timeline.selection;

  const [trackRef, setTrackRef] = createSignal<HTMLDivElement>();
  const trackBounds = createElementBounds(trackRef);

  let dragState: SceneSegmentDragState = { type: "idle" };

  const sceneSegments = () => project.timeline?.sceneSegments || [];

  const addSceneSegment = (mode: SceneMode) => {
    const time = editorState.playbackTime;
    const newSegment: SceneSegment = {
      start: time,
      end: Math.min(time + 2, editorState.timeline.transform.zoom),
      mode,
    };

    batch(() => {
      setProject(
        "timeline",
        "sceneSegments",
        produce((segments) => {
          if (!segments) {
            segments = [];
          }
          segments.push(newSegment);
          segments.sort((a, b) => a.start - b.start);
          return segments;
        })
      );
    });
  };

  return (
    <div class="relative h-10 border-t border-gray-7 bg-gray-1">
      <div class="absolute left-0 top-0 flex items-center gap-2 px-2 py-1 text-xs text-gray-9">
        <span>Scene</span>
        <button
          class="px-2 py-0.5 bg-gray-3 hover:bg-gray-4 rounded"
          onClick={() => addSceneSegment("default")}
        >
          Default
        </button>
        <button
          class="px-2 py-0.5 bg-gray-3 hover:bg-gray-4 rounded"
          onClick={() => addSceneSegment("cameraOnly")}
        >
          Camera Only
        </button>
        <button
          class="px-2 py-0.5 bg-gray-3 hover:bg-gray-4 rounded"
          onClick={() => addSceneSegment("hideCamera")}
        >
          Hide Camera
        </button>
      </div>
      <div
        ref={setTrackRef}
        class="relative h-full"
        onMouseDown={(e) => {
          if (dragState.type !== "idle") return;
          createRoot((dispose) => {
            createEventListener(e.currentTarget, "mouseup", () => {
              props.handleUpdatePlayhead(e);
              if (dragState.type === "idle") {
                setEditorState("timeline", "selection", null);
              }
              dispose();
            });
          });
        }}
      >
        <For each={sceneSegments()}>
          {(segment, index) => {
            const isSelected = () =>
              selection()?.type === "scene" &&
              (selection() as any).index === index();

            const segmentPixelStart = () =>
              (segment.start - transform().position) / secsPerPixel();
            const segmentPixelEnd = () =>
              (segment.end - transform().position) / secsPerPixel();
            const segmentPixelWidth = () => segmentPixelEnd() - segmentPixelStart();

            const modeColor = () => {
              switch (segment.mode) {
                case "cameraOnly":
                  return "bg-blue-500/30 border-blue-500";
                case "hideCamera":
                  return "bg-red-500/30 border-red-500";
                default:
                  return "bg-green-500/30 border-green-500";
              }
            };

            const modeLabel = () => {
              switch (segment.mode) {
                case "cameraOnly":
                  return "Camera";
                case "hideCamera":
                  return "No Cam";
                default:
                  return "Default";
              }
            };

            return (
              <Show when={segmentPixelEnd() > 0 && segmentPixelStart() < (trackBounds.width ?? 0)}>
                <div
                  class={`absolute top-1 bottom-1 border rounded ${modeColor()} ${
                    isSelected() ? "ring-2 ring-blue-400" : ""
                  }`}
                  style={{
                    left: `${segmentPixelStart()}px`,
                    width: `${segmentPixelWidth()}px`,
                  }}
                  onMouseDown={(e) => {
                    e.stopPropagation();
                    const startMouseX = e.clientX;
                    const startSegment = { ...segment };

                    // Determine if clicking on edges for resizing
                    const relativeX = e.clientX - e.currentTarget.getBoundingClientRect().left;
                    const edgeThreshold = 10;

                    if (relativeX < edgeThreshold) {
                      dragState = { type: "resizing-start", index: index(), startMouseX, startSegment };
                    } else if (relativeX > segmentPixelWidth() - edgeThreshold) {
                      dragState = { type: "resizing-end", index: index(), startMouseX, startSegment };
                    } else {
                      dragState = { type: "moving", index: index(), startMouseX, startSegment };
                    }

                    props.onDragStateChanged?.(dragState);
                    setEditorState("timeline", "selection", { type: "scene", index: index() } as any);

                    createRoot((dispose) => {
                      createEventListener(window, "mousemove", (e) => {
                        const deltaX = e.clientX - startMouseX;
                        const deltaTime = deltaX * secsPerPixel();

                        batch(() => {
                          if (dragState.type === "moving") {
                            setProject(
                              "timeline",
                              "sceneSegments",
                              index(),
                              produce((s) => {
                                s.start = Math.max(0, startSegment.start + deltaTime);
                                s.end = startSegment.end + deltaTime;
                              })
                            );
                          } else if (dragState.type === "resizing-start") {
                            setProject(
                              "timeline",
                              "sceneSegments",
                              index(),
                              "start",
                              Math.max(0, Math.min(startSegment.start + deltaTime, segment.end - 0.1))
                            );
                          } else if (dragState.type === "resizing-end") {
                            setProject(
                              "timeline",
                              "sceneSegments",
                              index(),
                              "end",
                              Math.max(segment.start + 0.1, startSegment.end + deltaTime)
                            );
                          }
                        });
                      });

                      createEventListener(window, "mouseup", () => {
                        dragState = { type: "idle" };
                        props.onDragStateChanged?.(dragState);
                        dispose();
                      });
                    });
                  }}
                >
                  <div class="px-1 text-xs text-white truncate">{modeLabel()}</div>
                  <Show when={segmentPixelWidth() > 10}>
                    <div class="absolute left-0 top-0 bottom-0 w-2 cursor-ew-resize" />
                    <div class="absolute right-0 top-0 bottom-0 w-2 cursor-ew-resize" />
                  </Show>
                </div>
              </Show>
            );
          }}
        </For>
      </div>
    </div>
  );
}