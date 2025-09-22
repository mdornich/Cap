import { createElementBounds } from "@solid-primitives/bounds";
import { createEventListener } from "@solid-primitives/event-listener";
import { For, Show, batch, createRoot, createSignal, onMount, onCleanup } from "solid-js";
import { produce } from "solid-js/store";
import { Menu } from "@tauri-apps/api/menu";

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

  const sceneSegments = () => {
    const segments = project.timeline?.sceneSegments || [];
    console.log("Current sceneSegments from project:", segments);
    return segments;
  };

  const deleteSceneSegment = (index: number) => {
    console.log("Deleting scene segment at index:", index);
    setProject(
      "timeline",
      "sceneSegments",
      (prevSegments) => {
        const segments = [...(prevSegments || [])];
        segments.splice(index, 1);
        console.log("Segments after deletion:", segments);
        return segments;
      }
    );
    // Clear selection after deletion
    setEditorState("timeline", "selection", null);
  };

  // Add keyboard shortcut for deleting selected segment
  onMount(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Check if Delete or Backspace key is pressed
      if (e.key === 'Delete' || e.key === 'Backspace') {
        // Check if a scene segment is selected
        const sel = selection();
        if (sel && 'type' in sel && sel.type === 'scene' && 'index' in sel) {
          e.preventDefault();
          deleteSceneSegment(sel.index);
        }
      }
    };
    
    document.addEventListener('keydown', handleKeyDown);
    onCleanup(() => document.removeEventListener('keydown', handleKeyDown));
  });

  const addSceneSegment = (mode: SceneMode) => {
    console.log("Scene button clicked:", mode);
    console.log("Current project.timeline:", project.timeline);
    
    if (!project.timeline) {
      console.error("No timeline found in project!");
      return;
    }
    
    const time = editorState.playbackTime;
    console.log("Playback time:", time);
    console.log("Zoom value:", editorState.timeline.transform.zoom);
    console.log("Actual recording duration:", project.timeline?.duration);
    
    // Create a 1-second segment from current playback time
    const segmentDuration = 1; // 1 second segment
    const recordingDuration = project.timeline?.duration || editorState.timeline.transform.zoom;
    const newSegment: SceneSegment = {
      start: time,
      end: Math.min(time + segmentDuration, recordingDuration),
      mode,
    };
    console.log("Creating new segment:", newSegment);
    console.log("Recording duration:", recordingDuration);

    batch(() => {
      // Ensure timeline exists
      if (!project.timeline) {
        setProject("timeline", {});
      }
      
      // Now update the sceneSegments
      setProject(
        "timeline",
        "sceneSegments", 
        (prevSegments) => {
          const segments = [...(prevSegments || [])];
          segments.push(newSegment);
          segments.sort((a, b) => a.start - b.start);
          console.log("Updated segments:", segments);
          console.log("Segments length:", segments.length);
          return segments;
        }
      );
      
      // Log the project after update
      setTimeout(() => {
        console.log("Project after update:", project);
        console.log("Timeline after update:", project.timeline);
        console.log("SceneSegments after update:", project.timeline?.sceneSegments);
      }, 100);
    });
  };

  return (
    <>
      {/* Scene track buttons - outside the timeline event capture area */}
      <div 
        class="relative flex items-center gap-2 px-2 py-1 text-xs text-gray-9 bg-gray-1 border-t border-gray-7"
        onMouseDown={(e) => e.stopPropagation()}>
        <span>Scene</span>
        <button
          class="px-2 py-0.5 bg-blue-500 hover:bg-blue-600 text-white rounded cursor-pointer"
          onClick={() => addSceneSegment("default")}
        >
          Default
        </button>
        <button
          class="px-2 py-0.5 bg-green-500 hover:bg-green-600 text-white rounded cursor-pointer"
          onClick={() => addSceneSegment("cameraOnly")}
        >
          Camera Only
        </button>
        <button
          class="px-2 py-0.5 bg-red-500 hover:bg-red-600 text-white rounded cursor-pointer"
          onClick={() => addSceneSegment("hideCamera")}
        >
          Hide Camera
        </button>
      </div>
      {/* Scene track area */}
      <div class="relative h-10 border-t border-gray-7 bg-gray-1">
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

            console.log(`Segment ${index()}: start=${segment.start}, end=${segment.end}, pixelStart=${segmentPixelStart()}, pixelEnd=${segmentPixelEnd()}`);
            
            return (
              <Show when={segmentPixelEnd() > 0 && segmentPixelStart() < (trackBounds.width ?? 0)}>
                <div
                  class={`absolute top-1 bottom-1 border-2 rounded ${modeColor()} ${
                    isSelected() ? "ring-2 ring-blue-400" : ""
                  } min-w-[20px]`}
                  style={{
                    left: `${segmentPixelStart()}px`,
                    width: `${segmentPixelWidth()}px`,
                  }}
                  onContextMenu={async (e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    
                    // Select the segment
                    setEditorState("timeline", "selection", { type: "scene", index: index() } as any);
                    
                    // Create context menu
                    const menu = await Menu.new({
                      id: "scene-segment-menu",
                      items: [
                        {
                          id: "delete",
                          text: "Delete Scene Segment",
                          action: () => deleteSceneSegment(index()),
                        },
                      ],
                    });
                    await menu.popup();
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
                  <div class="px-1 text-xs text-white truncate select-none">{modeLabel()}</div>
                  <Show when={isSelected()}>
                    <button
                      class="absolute top-0 right-0 m-0.5 p-0.5 bg-red-500 hover:bg-red-600 rounded text-white"
                      onClick={(e) => {
                        e.stopPropagation();
                        deleteSceneSegment(index());
                      }}
                      title="Delete segment (Delete/Backspace)"
                    >
                      <svg width="10" height="10" viewBox="0 0 10 10" fill="currentColor">
                        <path d="M2 2L8 8M8 2L2 8" stroke="currentColor" stroke-width="1.5"/>
                      </svg>
                    </button>
                  </Show>
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
    </>
  );
}