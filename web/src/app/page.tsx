import { ArrowRight, ArrowUpRight, Download } from "lucide-react";
import { Logo } from "@/components/logo";
import { SiteHeader } from "@/components/site-header";
import { HeroWindow } from "@/components/hero-window";
import { DownloadButton } from "@/components/download-button";
import { DemoPlayer } from "@/components/demo-player";
import { Gallery } from "@/components/gallery";
import { downloads, moduleGroups, REPO, VERSION } from "@/content/site";
import { cn } from "@/lib/utils";

const wrap = "mx-auto w-[min(100%-2rem,72rem)] md:w-[min(100%-4rem,72rem)]";
const h2 = "text-balance text-[clamp(1.85rem,3.6vw,2.75rem)] font-semibold leading-[1.08] tracking-[-0.028em]";
const lede = "mt-4 max-w-[60ch] text-pretty text-[17px] leading-relaxed text-muted-foreground";

export default function Home() {
  return (
    <>
      <a
        href="#contenido"
        className="sr-only z-50 rounded-md bg-foreground px-3 py-2 text-background focus:not-sr-only focus:fixed focus:left-4 focus:top-3"
      >
        Saltar al contenido
      </a>
      <SiteHeader />

      <main id="contenido">
        {/* Hero */}
        <section id="inicio" className="overflow-x-clip pb-10 pt-16 md:pt-20">
          <div className={cn(wrap, "flex flex-col items-center text-center")}>
            <h1 className="max-w-[16ch] text-balance text-[clamp(2.4rem,6.4vw,4.6rem)] font-semibold leading-[1.02] tracking-[-0.035em] md:max-w-none">
              Termodinámica aplicada,
              <br className="hidden md:block" />{" "}
              <span className="text-muted-foreground">calculada mientras la exploras.</span>
            </h1>
            <p className="mt-6 max-w-[58ch] text-pretty text-[clamp(1rem,1.5vw,1.15rem)] leading-relaxed text-muted-foreground">
              Equilibrio químico, agua y vapor, reacciones, balances de materia y energía y procesamiento mineral en veinte
              módulos de escritorio. Gratis para tus clases, prácticas e investigación.
            </p>
            <div className="mt-8 flex flex-col items-center gap-5 sm:flex-row sm:items-start">
              <DownloadButton extra={`v${VERSION} · código abierto (MIT)`} />
              <a
                href="#en-accion"
                className="inline-flex h-12 items-center gap-2 text-[15px] font-medium underline decoration-border decoration-1 underline-offset-[6px] transition-colors hover:text-brand hover:decoration-brand"
              >
                Verlo en acción
                <ArrowRight className="size-4" aria-hidden="true" />
              </a>
            </div>
          </div>
          <div className={cn(wrap, "mt-12 md:mt-14")}>
            <HeroWindow />
          </div>
        </section>

        {/* In action */}
        <section id="en-accion" className="py-[clamp(4.5rem,10vw,8rem)]">
          <div className={wrap}>
            <div className="mb-12 max-w-2xl">
              <h2 className={h2}>No son maquetas. Es el programa calculando.</h2>
              <p className={lede}>
                Cada clip es una grabación de la versión {VERSION} compilada. Lo que ves es lo que obtienes al
                instalarla.
              </p>
            </div>
            <DemoPlayer />
          </div>
        </section>

        {/* Classroom */}
        <section aria-labelledby="aula-title" className="border-y border-border bg-card py-[clamp(4rem,8vw,6.5rem)]">
          <div className={cn(wrap, "grid gap-10 md:grid-cols-[1fr_1.15fr] md:gap-16")}>
            <div>
              <h2 id="aula-title" className={h2}>
                Pensado para el aula y el laboratorio.
              </h2>
              <p className={lede}>
                Del ejercicio de clase al trabajo de tesis: los mismos modelos, con las fuentes de cada dato a la vista y sin
                licencias que gestionar.
              </p>
            </div>
            <dl className="divide-y divide-border border-y border-border">
              {[
                ["Prácticas de termodinámica", "Propiedades del agua y el vapor por IF97 en cualquier punto, sin interpolar en tablas."],
                ["Metalurgia extractiva", "Balances de calor y materia por elemento, como el de un convertidor de cobre."],
                ["Química física", "ΔH, ΔS, ΔG y constantes de equilibrio frente a temperatura, con su referencia bibliográfica."],
                ["Investigación", "Estimación de propiedades por grupos de Benson a partir de un SMILES."],
              ].map(([t, d]) => (
                <div key={t} className="grid gap-1 py-5 sm:grid-cols-[12rem_1fr] sm:gap-6">
                  <dt className="font-medium">{t}</dt>
                  <dd className="m-0 text-muted-foreground">{d}</dd>
                </div>
              ))}
            </dl>
          </div>
        </section>

        {/* Modules */}
        <section id="modulos" className="py-[clamp(4.5rem,10vw,8rem)]">
          <div className={wrap}>
            <div className="mb-12 max-w-2xl">
              <h2 className={h2}>Del dato termodinámico al circuito de planta.</h2>
              <p className={lede}>
                Cada módulo abre en su propia ventana y comparte la misma base de especies. Usa uno o combina varios en el mismo
                estudio.
              </p>
            </div>
            <Gallery />
            <div className="mt-20 grid gap-10 border-t border-border pt-10 sm:grid-cols-2 lg:grid-cols-4">
              {moduleGroups.map((g) => (
                <div key={g.title}>
                  <h3 className="text-sm font-medium text-muted-foreground">{g.title}</h3>
                  <ul className="mt-4 grid gap-2.5 text-[15px]">
                    {g.items.map((i) => (
                      <li key={i}>{i}</li>
                    ))}
                  </ul>
                </div>
              ))}
            </div>
          </div>
        </section>

        {/* Validation */}
        <section id="validacion" className="border-t border-border py-[clamp(4.5rem,10vw,8rem)]">
          <div className={cn(wrap, "grid items-start gap-12 md:grid-cols-2 md:gap-20")}>
            <div>
              <h2 className={h2}>La confianza empieza por declarar los límites.</h2>
              <p className={lede}>
                El repositorio documenta casos de referencia, pruebas automatizadas y el alcance de cada modelo. Validar casos
                concretos no equivale a calibrar una planta ni a ser equivalente a una base de datos comercial.
              </p>
              <a
                href={`${REPO}/blob/main/docs/SCIENTIFIC_VALIDATION.md`}
                target="_blank"
                rel="noopener noreferrer"
                className="mt-7 inline-flex items-center gap-1.5 font-medium underline decoration-border underline-offset-[6px] transition-colors hover:text-brand hover:decoration-brand"
              >
                Leer la cobertura científica
                <ArrowUpRight className="size-4" aria-hidden="true" />
              </a>
            </div>
            <dl className="divide-y divide-border border-y border-border">
              {[
                ["Casos de referencia", "Agua y vapor, equilibrio de Gibbs, equilibrio acuoso y conservación en balances minerales."],
                ["Procedencia visible", "Cada dato termodinámico muestra su fuente, como Barin o JANAF, junto al valor."],
                ["Alcance explícito", "Los modelos aproximados y los datos sin verificar se señalan como tales."],
              ].map(([t, d]) => (
                <div key={t} className="py-6">
                  <dt className="text-[17px] font-medium">{t}</dt>
                  <dd className="m-0 mt-1 text-muted-foreground">{d}</dd>
                </div>
              ))}
            </dl>
          </div>
        </section>

        {/* Download */}
        <section
          id="descargar"
          aria-labelledby="dl-title"
          className="bg-[oklch(0.2_0.02_170)] py-[clamp(4.5rem,10vw,8rem)] text-[oklch(0.95_0.006_160)]"
        >
          <div className={wrap}>
            <div className="mx-auto max-w-xl text-center">
              <h2 id="dl-title" className={h2}>
                Instálalo en un minuto.
              </h2>
              <p className="mt-4 text-pretty text-[17px] leading-relaxed text-[oklch(0.95_0.006_160/0.7)]">
                Un ejecutable nativo escrito en Rust, sin runtimes que instalar. Versión {VERSION} para x86‑64.
              </p>
            </div>
            <div className="mx-auto mt-12 grid max-w-3xl gap-5 md:grid-cols-2">
              {downloads.map((d) => (
                <div key={d.os} className="rounded-2xl bg-[oklch(1_0_0/0.04)] p-2 shadow-[0_0_0_1px_oklch(1_0_0/0.1)]">
                  <div className="flex items-baseline justify-between px-3.5 pb-3 pt-3.5">
                    <h3 className="text-xl font-semibold tracking-[-0.01em]">{d.os}</h3>
                    <span className="text-xs text-[oklch(0.95_0.006_160/0.6)]">{d.detail}</span>
                  </div>
                  {d.files.map((f) => (
                    <a
                      key={f.file}
                      href={f.href}
                      className={cn(
                        "group flex items-center justify-between gap-3 rounded-xl p-3.5 transition-colors",
                        f.primary
                          ? "bg-[oklch(0.95_0.006_160)] text-[oklch(0.2_0.02_170)] hover:bg-white"
                          : "hover:bg-[oklch(1_0_0/0.07)]",
                      )}
                    >
                      <span className="grid min-w-0 gap-0.5">
                        <span className="font-medium">{f.label}</span>
                        <span className="truncate font-mono text-xs opacity-60">{f.file}</span>
                      </span>
                      <span className="tabular flex shrink-0 items-center gap-2 text-sm opacity-75">
                        {f.size}
                        <Download className="size-4" aria-hidden="true" />
                      </span>
                    </a>
                  ))}
                </div>
              ))}
            </div>
            <p className="mt-8 text-center text-sm text-[oklch(0.95_0.006_160/0.6)]">
              ¿Otra versión o el código fuente?{" "}
              <a href={`${REPO}/releases`} className="underline underline-offset-4 hover:text-white" target="_blank" rel="noopener noreferrer">
                Todas las publicaciones
              </a>
            </p>
          </div>
        </section>
      </main>

      <footer className="bg-[oklch(0.2_0.02_170)] text-sm text-[oklch(0.95_0.006_160/0.6)]">
        <div className={cn(wrap, "flex flex-wrap items-center justify-between gap-6 border-t border-[oklch(1_0_0/0.08)] py-8")}>
          <Logo variant="white" className="h-8" />
          <span>Software científico de escritorio, escrito en Rust.</span>
          <a href={REPO} target="_blank" rel="noopener noreferrer" className="inline-flex items-center gap-1 hover:text-white">
            Código fuente <ArrowUpRight className="size-3.5" aria-hidden="true" />
          </a>
        </div>
      </footer>
    </>
  );
}
