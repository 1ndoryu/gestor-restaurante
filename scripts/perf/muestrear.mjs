/* Muestreador de recursos PERF-VPS (169A-4, Fase C).
 * Cada 5 s ejecuta `coolify-manager container-stats --json` y añade una línea
 * JSONL con timestamp. Corre en paralelo al harness desde este PC.
 *
 * Uso:
 *   node scripts/perf/muestrear.mjs --sitio restaurante-perf \
 *     --duracion 300 --salida scripts/perf/resultados/<tier>-<escenario>.stats.jsonl
 */
import { execFile } from "node:child_process";
import { appendFileSync } from "node:fs";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);
const MANAGER = process.env.COOLIFY_MANAGER_EXE
  || "C:\\Users\\Owner\\AppData\\Local\\Temp\\opencode\\coolify-manager.exe"; // copia a salvo del sweep GloryTmpSweep (purga C:\tmp\glory-target)

function args() {
  const m = {};
  for (let i = 2; i < process.argv.length; i += 2) {
    m[process.argv[i].replace(/^--/, "")] = process.argv[i + 1];
  }
  return m;
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/* `container-stats` del manager solo devuelve el contenedor app; para medir
 * app+postgres se usa `docker stats` vía host-exec (solo lectura). */
const APP = "app-p0sk8swg8o8kcokko0w4ocwg";
const PG = "postgres-p0sk8swg8o8kcokko0w4ocwg";

async function unaMuestra() {
  const { stdout } = await execFileAsync(MANAGER, [
    "host-exec", "--command",
    `docker stats --no-stream --format "{{json .}}" ${APP} ${PG}`,
  ]);
  return stdout.trim().split("\n").filter(Boolean).map((l) => JSON.parse(l));
}

async function main() {
  const a = args();
  const sitio = a.sitio ?? "restaurante-perf";
  const duracion = parseInt(a.duracion ?? "300", 10);
  const intervalo = parseInt(a.intervalo ?? "5", 10);
  const salida = a.salida;
  if (!salida) throw new Error("falta --salida");
  const fin = Date.now() + duracion * 1000;
  let n = 0;
  while (Date.now() < fin) {
    const ts = new Date().toISOString();
    try {
      const stats = await unaMuestra();
      appendFileSync(salida, JSON.stringify({ ts, ok: true, stats }) + "\n");
    } catch (e) {
      appendFileSync(salida, JSON.stringify({ ts, ok: false, error: String(e).slice(0, 200) }) + "\n");
    }
    if (++n % 12 === 0) console.log(`muestras: ${n} (${ts})`);
    await sleep(intervalo * 1000);
  }
  console.log(`fin: ${n} muestras en ${salida}`);
}

main().catch((e) => { console.error("MUESTREADOR-ERROR:", e.message); process.exit(1); });
