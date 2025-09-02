import { For, Show, createSignal, createEffect, batch } from "solid-js";
import { cx } from "cva";
import { useEditorContext, FPS, OUTPUT_SIZE } from "../context";
import { useTimelineContext, useTrackContext, TrackContextProvider } from "./context";
import { formatTime } from "../utils";
import { events } from "~/utils/tauri";

interface CaptionSegmentProps {
  segment: {
    id: string;
    start: number;
    end: number;
    text: string;
  };
  index: number;
  isSelected: boolean;
  onSelect: () => void;
}

function CaptionSegment(props: CaptionSegmentProps) {
  const { editorState, setEditorState, project, setProject } = useEditorContext();
  const { secsPerPixel, trackBounds } = useTrackContext();
  
  const [isDragging, setIsDragging] = createSignal<"start" | "end" | "move" | null>(null);
  const [isEditing, setIsEditing] = createSignal(false);
  const [editText, setEditText] = createSignal(props.segment.text);
  
  // Watch for external changes to the segment text
  createEffect(() => {
    if (!isEditing()) {
      setEditText(props.segment.text);
    }
  });
  
  // Stop editing when segment is deselected
  createEffect(() => {
    if (!props.isSelected && isEditing()) {
      setIsEditing(false);
    }
  });
  
  // Calculate pixel positions
  const startX = () => (props.segment.start - editorState.timeline.transform.position) / secsPerPixel();
  const width = () => (props.segment.end - props.segment.start) / secsPerPixel();
  
  // Check if segment is visible in viewport
  const isVisible = () => {
    const endX = startX() + width();
    return endX > 0 && startX() < (trackBounds.width ?? 0);
  };
  
  // Check if this is the current caption being played
  const isCurrent = () => {
    const time = editorState.playbackTime;
    return time >= props.segment.start && time < props.segment.end;
  };
  
  const handleMouseDown = (e: MouseEvent, type: "start" | "end" | "move") => {
    e.preventDefault();
    e.stopPropagation();
    
    if (isEditing()) return;
    
    props.onSelect();
    setIsDragging(type);
    
    const startPos = e.clientX;
    const startSegment = { ...props.segment };
    
    const handleMouseMove = (e: MouseEvent) => {
      const delta = (e.clientX - startPos) * secsPerPixel();
      
      if (!project?.captions?.segments) return;
      
      batch(() => {
        const segments = [...project.captions.segments];
        const segmentIndex = segments.findIndex(s => s.id === props.segment.id);
        
        if (segmentIndex === -1) return;
        
        if (type === "start") {
          const newStart = Math.max(0, Math.min(startSegment.start + delta, startSegment.end - 0.1));
          segments[segmentIndex] = { ...segments[segmentIndex], start: newStart };
        } else if (type === "end") {
          const newEnd = Math.max(startSegment.start + 0.1, startSegment.end + delta);
          segments[segmentIndex] = { ...segments[segmentIndex], end: newEnd };
        } else if (type === "move") {
          const duration = startSegment.end - startSegment.start;
          const newStart = Math.max(0, startSegment.start + delta);
          segments[segmentIndex] = {
            ...segments[segmentIndex],
            start: newStart,
            end: newStart + duration
          };
        }
        
        setProject("captions", "segments", segments);
      });
    };
    
    const handleMouseUp = () => {
      setIsDragging(null);
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
      
      // Force frame re-render after drag to prevent black screen
      setTimeout(() => {
        events.renderFrameEvent.emit({
          frame_number: Math.floor(editorState.playbackTime * FPS),
          fps: FPS,
          resolution_base: OUTPUT_SIZE,
        });
      }, 100);
    };
    
    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
  };
  
  const handleDoubleClick = (e: MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsEditing(true);
    setEditText(props.segment.text);
    
    // Force frame re-render when starting edit to prevent black screen
    setTimeout(() => {
      events.renderFrameEvent.emit({
        frame_number: Math.floor(editorState.playbackTime * FPS),
        fps: FPS,
        resolution_base: OUTPUT_SIZE,
      });
    }, 100);
  };
  
  const saveEdit = () => {
    if (!project?.captions?.segments) return;
    
    // Update only the text of the specific segment, maintaining order
    const updatedSegments = [...project.captions.segments];
    const segmentIndex = updatedSegments.findIndex(s => s.id === props.segment.id);
    
    if (segmentIndex !== -1) {
      updatedSegments[segmentIndex] = {
        ...updatedSegments[segmentIndex],
        text: editText()
      };
      
      setProject("captions", "segments", updatedSegments);
      
      // Force frame re-render after a delay to prevent black screen
      setTimeout(() => {
        events.renderFrameEvent.emit({
          frame_number: Math.floor(editorState.playbackTime * FPS),
          fps: FPS,
          resolution_base: OUTPUT_SIZE,
        });
      }, 200);
    }
    
    setIsEditing(false);
  };
  
  return (
    <Show when={isVisible()}>
      <div
        class={cx(
          "absolute top-2 h-12 rounded-md transition-all cursor-pointer",
          "hover:shadow-lg hover:z-10",
          props.isSelected && "ring-2 ring-blue-500 z-20",
          isCurrent() && "bg-blue-500/90 text-white",
          !isCurrent() && "bg-gray-400 dark:bg-gray-700 text-white",
          isDragging() && "opacity-75"
        )}
        style={{
          left: `${startX()}px`,
          width: `${width()}px`,
        }}
        onClick={(e) => {
          e.stopPropagation();
          
          // If clicking on a different segment while another is being edited
          if (!props.isSelected) {
            // First, close any other editing segments by triggering selection
            props.onSelect();
          } else if (!isEditing()) {
            // If this segment is selected but not editing, start editing
            setIsEditing(true);
            setEditText(props.segment.text);
            
            // Schedule focus after render
            setTimeout(() => {
              const input = e.currentTarget.querySelector('input');
              if (input) {
                input.focus();
                input.select();
              }
            }, 50);
          }
          
          // Force frame re-render when clicking on segment to prevent black screen
          setTimeout(() => {
            events.renderFrameEvent.emit({
              frame_number: Math.floor(editorState.playbackTime * FPS),
              fps: FPS,
              resolution_base: OUTPUT_SIZE,
            });
          }, 100);
        }}
        onDblClick={handleDoubleClick}
        onMouseDown={(e) => handleMouseDown(e, "move")}
      >
        {/* Resize handles */}
        <div
          class="absolute left-0 top-0 bottom-0 w-2 cursor-ew-resize hover:bg-blue-400/50"
          onMouseDown={(e) => handleMouseDown(e, "start")}
        />
        <div
          class="absolute right-0 top-0 bottom-0 w-2 cursor-ew-resize hover:bg-blue-400/50"
          onMouseDown={(e) => handleMouseDown(e, "end")}
        />
        
        {/* Content */}
        <div class="px-3 py-1 h-full flex items-center overflow-hidden">
          <Show
            when={!isEditing()}
            fallback={
              <input
                type="text"
                value={editText()}
                onInput={(e) => {
                  setEditText(e.currentTarget.value);
                  
                  // Force frame re-render while typing to prevent black screen
                  setTimeout(() => {
                    events.renderFrameEvent.emit({
                      frame_number: Math.floor(editorState.playbackTime * FPS),
                      fps: FPS,
                      resolution_base: OUTPUT_SIZE,
                    });
                  }, 50);
                }}
                onKeyDown={(e) => {
                  // Stop propagation for ALL keys when editing to prevent global handlers
                  e.stopPropagation();
                  e.stopImmediatePropagation();
                  
                  if (e.key === "Enter") {
                    e.preventDefault(); // Prevent form submission if any
                    saveEdit();
                  } else if (e.key === "Escape") {
                    e.preventDefault();
                    setIsEditing(false);
                    
                    // Force frame re-render when canceling edit to prevent black screen
                    setTimeout(() => {
                      events.renderFrameEvent.emit({
                        frame_number: Math.floor(editorState.playbackTime * FPS),
                        fps: FPS,
                        resolution_base: OUTPUT_SIZE,
                      });
                    }, 100);
                  }
                  // For all other keys including Delete/Backspace, let default behavior happen
                }}
                onKeyUp={(e) => {
                  e.stopPropagation();
                  e.stopImmediatePropagation();
                }}
                onKeyPress={(e) => {
                  e.stopPropagation();
                  e.stopImmediatePropagation();
                }}
                onBlur={() => {
                  saveEdit();
                  // Extra re-render on blur
                  setTimeout(() => {
                    events.renderFrameEvent.emit({
                      frame_number: Math.floor(editorState.playbackTime * FPS),
                      fps: FPS,
                      resolution_base: OUTPUT_SIZE,
                    });
                  }, 300);
                }}
                onFocus={(e) => {
                  e.currentTarget.select();
                  // Re-render on focus
                  setTimeout(() => {
                    events.renderFrameEvent.emit({
                      frame_number: Math.floor(editorState.playbackTime * FPS),
                      fps: FPS,
                      resolution_base: OUTPUT_SIZE,
                    });
                  }, 100);
                }}
                class="w-full bg-gray-800 text-white px-1 rounded border border-gray-600"
                autofocus
                onClick={(e) => {
                  e.stopPropagation();
                  e.stopImmediatePropagation();
                }}
              />
            }
          >
            <span class="text-xs truncate select-none">
              {props.segment.text}
            </span>
          </Show>
        </div>
        
        {/* Time labels */}
        <div class="absolute -top-5 left-0 text-[10px] text-gray-500 select-none">
          {formatTime(props.segment.start)}
        </div>
      </div>
    </Show>
  );
}

export function CaptionTrack() {
  const { project, setProject, editorState, setEditorState } = useEditorContext();
  const [trackRef, setTrackRef] = createSignal<HTMLDivElement>();
  
  const handleAddCaption = () => {
    if (!project?.captions) return;
    
    const time = editorState.playbackTime;
    const id = `caption-${Date.now()}`;
    const newSegment = {
      id,
      start: time,
      end: time + 2,
      text: "New caption"
    };
    
    // Keep existing segments and add new one
    const segments = [...(project.captions.segments || []), newSegment];
    // Sort to maintain chronological order
    segments.sort((a, b) => a.start - b.start);
    
    batch(() => {
      setProject("captions", "segments", segments);
      setEditorState("timeline", "selection", { type: "caption", id });
    });
    
    // Force frame re-render after adding caption to prevent black screen
    setTimeout(() => {
      events.renderFrameEvent.emit({
        frame_number: Math.floor(editorState.playbackTime * FPS),
        fps: FPS,
        resolution_base: OUTPUT_SIZE,
      });
    }, 100);
  };
  
  return (
    <TrackContextProvider ref={trackRef}>
      <div class="relative w-full">
        {/* Track header */}
        <div class="flex items-center justify-between px-4 py-2 bg-gray-100 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
          <div class="flex items-center gap-2">
            <span class="text-sm font-medium text-gray-700 dark:text-gray-300">📝 Captions</span>
            <Show when={project?.captions?.segments?.length}>
              <span class="text-xs text-gray-500">
                ({project.captions.segments.length} segments)
              </span>
            </Show>
          </div>
          
          <button
            type="button"
            onClick={handleAddCaption}
            class="text-xs px-2 py-1 rounded bg-blue-500 text-white hover:bg-blue-600 transition-colors"
            title="Add caption at playhead (N)"
          >
            + Add Caption
          </button>
        </div>
        
        {/* Track content */}
        <div 
          ref={setTrackRef}
          class="relative h-16 bg-gray-50 dark:bg-gray-900 border-b border-gray-200 dark:border-gray-700 overflow-hidden"
          onClick={(e) => {
            // Force frame re-render when clicking anywhere in the track to prevent black screen
            setTimeout(() => {
              events.renderFrameEvent.emit({
                frame_number: Math.floor(editorState.playbackTime * FPS),
                fps: FPS,
                resolution_base: OUTPUT_SIZE,
              });
            }, 100);
          }}
          onMouseDown={(e) => {
            // Also re-render on mouse down
            setTimeout(() => {
              events.renderFrameEvent.emit({
                frame_number: Math.floor(editorState.playbackTime * FPS),
                fps: FPS,
                resolution_base: OUTPUT_SIZE,
              });
            }, 50);
          }}
          onMouseUp={(e) => {
            // And on mouse up
            setTimeout(() => {
              events.renderFrameEvent.emit({
                frame_number: Math.floor(editorState.playbackTime * FPS),
                fps: FPS,
                resolution_base: OUTPUT_SIZE,
              });
            }, 150);
          }}
        >
          <Show
            when={project?.captions?.segments && project.captions.segments.length > 0}
            fallback={
              <div class="flex items-center justify-center h-full text-gray-400 text-sm">
                No captions yet. Generate or add captions to see them here.
              </div>
            }
          >
            <For each={project.captions.segments}>
              {(segment, index) => (
                <CaptionSegment
                  segment={segment}
                  index={index()}
                  isSelected={
                    editorState.timeline.selection?.type === "caption" &&
                    editorState.timeline.selection.id === segment.id
                  }
                  onSelect={() => {
                    setEditorState("timeline", "selection", { type: "caption", id: segment.id });
                    
                    // Force frame re-render when selecting segment to prevent black screen
                    setTimeout(() => {
                      events.renderFrameEvent.emit({
                        frame_number: Math.floor(editorState.playbackTime * FPS),
                        fps: FPS,
                        resolution_base: OUTPUT_SIZE,
                      });
                    }, 100);
                  }}
                />
              )}
            </For>
          </Show>
        </div>
      </div>
    </TrackContextProvider>
  );
}