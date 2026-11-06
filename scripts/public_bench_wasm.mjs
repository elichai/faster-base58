// Wasm worker for public_bench.py: same job and result format as its native worker.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { openProbe } from "./code_size_wasm.mjs";

const job = JSON.parse(process.argv[2]);
const supported = (label, size) => {
    const backend = label.split("+")[0];
    if (backend === "five8") return size === 32 || size === 64;
    return backend === "no-alloc" || backend === "bs58" ? size <= 65536 : true;
};
const probes = new Map();
for (const [label, path] of job.libraries) {
    const probe = await openProbe(path);
    // Allocation counting adds work to every allocation, so it would distort timing.
    if (probe.exports.includes("size_count_begin")) throw new Error(`timing requires uninstrumented modules: ${path}`);
    probes.set(label, probe);
}

const results = [];
for (const size of job.sizes) {
    const data = new Uint8Array(readFileSync(join(job.fixtures, `${size}.bin`)));
    const labels = job.libraries.map(([label]) => label).filter(label => supported(label, size));
    if (!labels.length) continue;
    const text = probes.get(labels[0]).call("encode", data).output;
    // Decoding uses an exact buffer, which five8's fixed-width API requires.
    for (const [operation, value, expected, capacity] of [["encode", data, text, 2 * size + 16], ["decode", text, data, size]]) {
        for (const label of labels) assert.deepEqual(probes.get(label).call(operation, value, 1, capacity).output, expected, `${label} ${operation} ${size}`);
        const batch = (label, count) => probes.get(label).call(operation, value, count, capacity).elapsedNs;
        const counts = new Map();
        for (const label of labels) {
            let count = 1;
            while (batch(label, count) < job.min_ns) count *= 2;
            counts.set(label, count);
        }
        const samples = new Map(labels.map(label => [label, []]));
        for (let round = 0; round < job.batches; round++) {
            const rotation = round % labels.length;
            for (const label of [...labels.slice(rotation), ...labels.slice(0, rotation)]) {
                samples.get(label).push(batch(label, counts.get(label)) / counts.get(label));
            }
        }
        for (const [label, times] of samples) {
            times.sort((a, b) => a - b);
            const middle = times.length >> 1;
            const ns = times.length % 2 ? times[middle] : (times[middle - 1] + times[middle]) / 2;
            results.push({ label, operation, size, ns, iterations: counts.get(label) });
        }
    }
}
console.log(JSON.stringify(results));
