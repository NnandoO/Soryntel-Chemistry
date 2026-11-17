"use client";

import { forwardRef, useEffect, useImperativeHandle, useRef } from "react";
import { cn } from "@/lib/utils";

type Props = {
  name: string;
  label: string;
  /** Play only while on screen; never when the visitor prefers reduced motion. */
  autoPlayInView?: boolean;
  loop?: boolean;
  /** object-position used for the narrower phone crop. */
  focus?: string;
  className?: string;
  onEnded?: () => void;
  onTime?: (fraction: number) => void;
  onPlayState?: (playing: boolean) => void;
};

/** A 60 fps H.264 recording of the app. */
export const ClipVideo = forwardRef<HTMLVideoElement | null, Props>(function ClipVideo(
  { name, label, autoPlayInView = true, loop = false, focus = "50% 50%", className, onEnded, onTime, onPlayState },
  ref,
) {
  const video = useRef<HTMLVideoElement>(null);
  useImperativeHandle(ref, () => video.current as HTMLVideoElement);

  useEffect(() => {
    const el = video.current;
    if (!el || !autoPlayInView) return;
    const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (reduce) return;
    const io = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) el.play().catch(() => {});
        else el.pause();
      },
      { threshold: 0.35 },
    );
    io.observe(el);
    return () => io.disconnect();
  }, [autoPlayInView, name]);

  useEffect(() => {
    const el = video.current;
    if (!el || !onTime) return;
    let raf = 0;
    const tick = () => {
      if (el.duration) onTime(el.currentTime / el.duration);
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [onTime, name]);

  return (
    <video
      key={name}
      ref={video}
      className={cn("block aspect-[1440/872] w-full bg-window object-cover max-sm:aspect-[4/5]", className)}
      style={{ objectPosition: focus }}
      muted
      playsInline
      loop={loop}
      preload="metadata"
      poster={`/media/${name}.jpg`}
      aria-label={label}
      onEnded={onEnded}
      onPlay={() => onPlayState?.(true)}
      onPause={() => onPlayState?.(false)}
    >
      <source src={`/media/${name}.mp4`} type="video/mp4" />
    </video>
  );
});
