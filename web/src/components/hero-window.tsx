"use client";

import { useEffect, useRef, useState } from "react";
import { Pause, Play } from "lucide-react";
import { AppWindow } from "@/components/app-window";
import { ClipVideo } from "@/components/clip-video";
import { clips } from "@/content/site";

const water = clips.find((c) => c.id === "water")!;

/** The page's one authored motion: the window settles flat as the visitor starts reading. */
export function HeroWindow() {
  const [flat, setFlat] = useState(false);
  const [playing, setPlaying] = useState(false);
  const video = useRef<HTMLVideoElement | null>(null);

  useEffect(() => {
    const on = () => setFlat(window.scrollY > 40);
    on();
    window.addEventListener("scroll", on, { passive: true });
    return () => window.removeEventListener("scroll", on);
  }, []);

  const toggle = () => {
    const el = video.current;
    if (!el) return;
    if (el.paused) el.play().catch(() => {});
    else el.pause();
  };

  return (
    <div className="[perspective:2200px]">
      <div
        className="origin-top transition-transform duration-[900ms] ease-[cubic-bezier(0.16,1,0.3,1)] motion-reduce:transform-none"
        style={{ transform: flat ? "none" : "rotateX(6deg) scale(0.985)" }}
      >
        <AppWindow
          title={water.window}
          footer={
            <figcaption className="flex items-baseline gap-3 border-t border-border bg-card px-4 py-3 text-sm text-muted-foreground">
              <span className="shrink-0 rounded-md bg-brand-soft px-2 py-1 text-xs font-medium text-brand">Grabación real</span>
              <span>{water.caption}</span>
            </figcaption>
          }
        >
          <div className="relative">
            <ClipVideo
              ref={video}
              name="water"
              loop
              focus={water.focus}
              onPlayState={setPlaying}
              label="Grabación del programa: al arrastrar un punto sobre el diagrama presión‑temperatura del agua, la tabla recalcula entalpía, entropía, densidad y exergía."
            />
            <button
              type="button"
              onClick={toggle}
              aria-label={playing ? "Pausar video" : "Reproducir video"}
              className="absolute right-3 top-3 grid size-9 cursor-pointer place-items-center rounded-full bg-card/90 text-foreground shadow-[0_0_0_1px_var(--border),0_4px_12px_-4px_oklch(0_0_0/0.25)] backdrop-blur-sm transition-colors hover:bg-card"
            >
              {playing ? <Pause className="size-3.5" aria-hidden="true" /> : <Play className="size-3.5" aria-hidden="true" />}
            </button>
          </div>
        </AppWindow>
      </div>
    </div>
  );
}
