"use client";

import { useRef } from "react";
import { ArrowLeft, ArrowRight } from "lucide-react";
import { stills } from "@/content/site";

/** Horizontal, snap-scrolling strip of module captures. */
export function Gallery() {
  const strip = useRef<HTMLDivElement>(null);
  const step = (dir: 1 | -1) => {
    const el = strip.current;
    const card = el?.querySelector("figure");
    if (!el || !card) return;
    el.scrollBy({ left: dir * (card.getBoundingClientRect().width + 20), behavior: "smooth" });
  };

  return (
    <div>
      <div
        ref={strip}
        className="no-scrollbar -mr-[max(1rem,calc((100vw_-_72rem)/2))] grid snap-x snap-mandatory auto-cols-[min(82%,42rem)] grid-flow-col gap-5 overflow-x-auto pb-1 pr-8"
        tabIndex={0}
        aria-label="Capturas de módulos"
      >
        {stills.map((s) => (
          <figure key={s.src} className="m-0 snap-start">
            {/* eslint-disable-next-line @next/next/no-img-element -- static export serves pre-sized WebP */}
            <img
              src={`/media/${s.src}.webp`}
              alt={s.alt}
              width={1440}
              height={872}
              loading="lazy"
              decoding="async"
              className="aspect-[1440/872] w-full rounded-xl bg-card object-cover object-left-top shadow-[0_0_0_1px_var(--border),0_18px_40px_-28px_oklch(0.22_0.02_170/0.45)]"
            />
            <figcaption className="mt-3.5 max-w-xl text-[15px] leading-relaxed text-muted-foreground">
              <strong className="mr-1.5 font-medium text-foreground">{s.title}.</strong>
              {s.text}
            </figcaption>
          </figure>
        ))}
      </div>
      <div className="mt-6 flex gap-2">
        {([-1, 1] as const).map((d) => (
          <button
            key={d}
            type="button"
            onClick={() => step(d)}
            aria-label={d < 0 ? "Captura anterior" : "Captura siguiente"}
            className="grid size-10 cursor-pointer place-items-center rounded-full bg-card text-foreground shadow-[0_0_0_1px_var(--border)] transition-colors hover:bg-foreground hover:text-background"
          >
            {d < 0 ? <ArrowLeft className="size-4" aria-hidden="true" /> : <ArrowRight className="size-4" aria-hidden="true" />}
          </button>
        ))}
      </div>
    </div>
  );
}
