"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { Pause, Play } from "lucide-react";
import { AppWindow } from "@/components/app-window";
import { ClipVideo } from "@/components/clip-video";
import { clips as allClips } from "@/content/site";

// The hero already plays the water clip; the player shows the other demos.
const clips = allClips.filter((c) => c.id !== "water");
import { cn } from "@/lib/utils";

/** Tabbed player: each clip plays through, then advances to the next. */
export function DemoPlayer() {
  const [index, setIndex] = useState(0);
  const [progress, setProgress] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [userPaused, setUserPaused] = useState(false);
  // With reduced motion the player waits for an explicit play.
  const interacted = useRef(false);
  const [visible, setVisible] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const video = useRef<HTMLVideoElement | null>(null);
  const tabs = useRef<(HTMLButtonElement | null)[]>([]);
  const clip = clips[index];

  useEffect(() => {
    const el = root.current;
    if (!el) return;
    const io = new IntersectionObserver(([e]) => setVisible(e.isIntersecting), { threshold: 0.3 });
    io.observe(el);
    return () => io.disconnect();
  }, []);

  // Play whenever the player is on screen, the clip changed, and the visitor has not paused.
  useEffect(() => {
    const el = video.current;
    if (!el) return;
    const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (visible && !userPaused && (!reduce || interacted.current)) el.play().catch(() => {});
    else el.pause();
  }, [visible, userPaused, index]);

  const select = useCallback((i: number, focus = false) => {
    setIndex(i);
    setProgress(0);
    if (focus) tabs.current[i]?.focus();
  }, []);

  const onKey = (e: React.KeyboardEvent, i: number) => {
    const step = { ArrowDown: 1, ArrowRight: 1, ArrowUp: -1, ArrowLeft: -1 }[e.key];
    if (step) {
      e.preventDefault();
      select((i + step + clips.length) % clips.length, true);
    } else if (e.key === "Home") {
      e.preventDefault();
      select(0, true);
    } else if (e.key === "End") {
      e.preventDefault();
      select(clips.length - 1, true);
    }
  };

  const toggle = () => {
    const el = video.current;
    if (!el) return;
    interacted.current = true;
    if (el.paused) {
      setUserPaused(false);
      el.play().catch(() => {});
    } else {
      setUserPaused(true);
      el.pause();
    }
  };

  return (
    <div ref={root} className="grid items-start gap-5 lg:grid-cols-[300px_1fr] lg:gap-10">
      <div
        role="tablist"
        aria-label="Demostraciones del programa"
        aria-orientation="vertical"
        className="no-scrollbar -mx-4 flex gap-1 overflow-x-auto px-4 lg:mx-0 lg:flex-col lg:overflow-visible lg:px-0"
      >
        {clips.map((c, i) => {
          const active = i === index;
          return (
            <button
              key={c.id}
              ref={(el) => {
                tabs.current[i] = el;
              }}
              role="tab"
              id={`tab-${c.id}`}
              aria-selected={active}
              aria-controls="demo-panel"
              tabIndex={active ? 0 : -1}
              onClick={() => {
                interacted.current = true;
                setUserPaused(false);
                select(i);
              }}
              onKeyDown={(e) => onKey(e, i)}
              className={cn(
                "relative min-w-40 shrink-0 cursor-pointer rounded-xl px-4 pb-5 pt-3.5 text-left transition-colors duration-200 lg:min-w-0",
                active ? "bg-card shadow-[0_0_0_1px_var(--border)]" : "hover:bg-card/60",
              )}
            >
              <span className={cn("block text-[15px] font-medium tracking-[-0.01em]", !active && "text-muted-foreground")}>
                {c.title}
              </span>
              <span
                className={cn(
                  "hidden text-sm leading-relaxed text-muted-foreground transition-[opacity,margin] duration-300 lg:grid",
                  active ? "mt-1 grid-rows-[1fr] opacity-100" : "grid-rows-[0fr] opacity-0",
                )}
              >
                <span className="overflow-hidden">{c.summary}</span>
              </span>
              <span
                aria-hidden="true"
                className={cn(
                  "absolute inset-x-4 bottom-2.5 h-0.5 overflow-hidden rounded-full bg-border transition-opacity",
                  active ? "opacity-100" : "opacity-0",
                )}
              >
                <span
                  className="block h-full origin-left rounded-full bg-brand"
                  style={{ transform: `scaleX(${active ? progress : 0})` }}
                />
              </span>
            </button>
          );
        })}
      </div>

      <AppWindow
        title={clip.window}
        footer={
          <figcaption className="border-t border-border bg-card px-4 py-3 text-sm text-muted-foreground">
            {clip.caption}
          </figcaption>
        }
      >
        <div id="demo-panel" role="tabpanel" aria-labelledby={`tab-${clip.id}`} className="relative">
          <ClipVideo
            ref={video}
            name={clip.id}
            focus={clip.focus}
            label={`Grabación del programa: ${clip.summary}`}
            autoPlayInView={false}
            onEnded={() => select((index + 1) % clips.length)}
            onTime={setProgress}
            onPlayState={setPlaying}
          />
          <button
            type="button"
            onClick={toggle}
            aria-label={playing ? "Pausar demostración" : "Reproducir demostración"}
            className="absolute bottom-3 right-3 grid size-9 cursor-pointer place-items-center rounded-full bg-card/90 text-foreground shadow-[0_0_0_1px_var(--border),0_4px_12px_-4px_oklch(0_0_0/0.25)] backdrop-blur-sm transition-colors hover:bg-card"
          >
            {playing ? <Pause className="size-3.5" aria-hidden="true" /> : <Play className="size-3.5" aria-hidden="true" />}
          </button>
        </div>
      </AppWindow>
    </div>
  );
}
