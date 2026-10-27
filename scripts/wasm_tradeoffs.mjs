import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { spawn } from "node:child_process";
import { mkdir, open, readFile, readdir, writeFile } from "node:fs/promises";
import os from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { openProbe } from "./code_size_wasm.mjs";

const SELF = fileURLToPath(import.meta.url);
const HELPER = join(dirname(SELF), "code_size_wasm.mjs");
const BACKENDS = ["empty", "auto", "batched", "karatsuba", "toom3", "ntt", "ntt-cached", "ntt-portable", "malachite", "matrix", "matrix-portable"];
const ALL_BACKENDS = [...BACKENDS, "no-alloc", "bs58", "five8", "matrix-forced", "ntt-same-owned", "ntt-same-cached"];
const SIZES = [0, 1, 32, 64, 512, 1024, 4096, 8192, 65536, 1048576];
const FLAGS = ["--no-liftoff", "--no-wasm-lazy-compilation", "--no-wasm-async-compilation"];
const SAMPLES = 3;
const MAX_BYTES = 4194304;
const TIMING_PROTOCOL = "base58-timing-20261024-v1";
const MIN_BATCH_NS = 3_000_000;
const MAX_ITERATIONS = 10_000_000;
const PAGE_BYTES = 65536;
const ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
const TEXT = new TextEncoder();
const hash = bytes => createHash("sha256").update(bytes).digest("hex");
const fileHash = async path => hash(await readFile(path));
const fnv1a64 = bytes => {
    let value = 0xcbf29ce484222325n;
    for (const byte of bytes) value = BigInt.asUintN(64, (value ^ BigInt(byte)) * 0x100000001b3n);
    return value.toString(16).padStart(16, "0");
};
const csv = values => values.map(value => {
    const text = String(value);
    return /[",\r\n]/.test(text) ? `"${text.replaceAll('"', '""')}"` : text;
}).join(",") + "\n";

function encodeReference(bytes) {
    let value = 0n;
    for (const byte of bytes) value = (value << 8n) | BigInt(byte);
    let digits = "";
    while (value) {
        digits = ALPHABET[Number(value % 58n)] + digits;
        value /= 58n;
    }
    let zeros = 0;
    while (zeros < bytes.length && bytes[zeros] === 0) zeros++;
    return "1".repeat(zeros) + digits;
}

function decodeReference(text) {
    let value = 0n;
    for (const character of text) {
        const digit = ALPHABET.indexOf(character);
        assert(digit >= 0, "invalid reference digit");
        value = value * 58n + BigInt(digit);
    }
    const bytes = [];
    while (value) {
        bytes.push(Number(value & 255n));
        value >>= 8n;
    }
    let zeros = 0;
    while (zeros < text.length && text[zeros] === "1") zeros++;
    return Uint8Array.from([...Array(zeros).fill(0), ...bytes.reverse()]);
}

function dataFor(size) {
    const bytes = new Uint8Array(size);
    let state = BigInt(size) + 20_261_023n;
    for (let i = 0; i < size; i++) {
        state = BigInt.asUintN(64, state ^ (state << 13n));
        state ^= state >> 7n;
        state = BigInt.asUintN(64, state ^ (state << 17n));
        bytes[i] = Number(state & 255n);
    }
    if (size) bytes[0] |= 1;
    return bytes;
}

function skipReason(backend, size, operation, encodedBytes) {
    if (backend === "five8" && size !== 32 && size !== 64) return "five8 supports only 32- and 64-byte decoded fixtures";
    if (["batched", "no-alloc", "bs58"].includes(backend) && size > 65536) return "quadratic backend capped at 65536 decoded bytes";
    if (backend === "matrix-forced") {
        if (size === 0) return "forced matrix control excludes the degenerate empty input";
        if (operation === "encode" && size > 8192) return "forced matrix encoder supports at most 8192 decoded bytes";
        if (operation === "decode" && encodedBytes > 5632) return "forced matrix decoder supports at most 5632 encoded characters; larger inputs use fallback";
    }
    return null;
}

function shuffledJobs(jobs, seed) {
    const result = [...jobs];
    let state = seed >>> 0;
    for (let index = result.length - 1; index > 0; index--) {
        state = (state ^ (state << 13)) >>> 0;
        state = (state ^ (state >>> 17)) >>> 0;
        state = (state ^ (state << 5)) >>> 0;
        const other = state % (index + 1);
        [result[index], result[other]] = [result[other], result[index]];
    }
    return result;
}

function backendRegistry(backends) {
    return backends.map(backend => {
        const maximum = ["batched", "no-alloc", "bs58"].includes(backend) ? 65536 : backend === "five8" ? 64 : MAX_BYTES;
        const row = { backend, max_encode_bytes: maximum, max_decode_bytes: maximum };
        if (backend === "five8") Object.assign(row, { supported_encode_bytes: [32, 64], supported_decode_bytes: [32, 64] });
        if (backend === "matrix-forced") Object.assign(row, {
            min_encode_bytes: 1, max_encode_bytes: 8192, min_decode_bytes: 1, max_decode_bytes: 5632,
            min_decode_chars: 1, max_decode_chars: 5632,
        });
        return row;
    });
}

async function inspectModules(directory, backends = BACKENDS) {
    const sizesBytes = await readFile(join(directory, "sizes.csv"));
    const [header, ...lines] = sizesBytes.toString().trim().split(/\r?\n/).map(line => line.split(","));
    const modules = [];
    const buildMetadata = JSON.parse(await readFile(join(directory, "metadata.json"), "utf8"));
    assert.equal(buildMetadata.count_allocations ?? false, false, "timing requires uninstrumented modules");
    const directions = ["encode", "decode", "both"].filter(direction => lines.some(row =>
        backends.includes(row[header.indexOf("backend")]) && row[header.indexOf("direction")] === direction));
    assert(directions.includes("both"), "timing requires both-direction modules");
    for (const backend of backends) {
        for (const direction of directions) {
            const matches = lines.filter(row => row[header.indexOf("backend")] === backend
                && row[header.indexOf("direction")] === direction);
            assert.equal(matches.length, 1, `${backend}/${direction}: expected one size row`);
            const row = Object.fromEntries(header.map((key, index) => [key, matches[0][index]]));
            assert.equal(row.target, "wasm32-unknown-unknown");
            assert.equal(row.kind, "cdylib");
            const path = join(directory, `${backend}-${direction}-cdylib.wasm.stripped`);
            const bytes = await readFile(path);
            assert(WebAssembly.validate(bytes), `invalid module: ${path}`);
            assert(!WebAssembly.Module.exports(new WebAssembly.Module(bytes)).some(entry => entry.name === "size_count_begin"),
                   "timing requires uninstrumented modules: allocation hooks exported");
            assert.equal(bytes.length, Number(row.stripped_file_bytes));
            assert.equal(hash(bytes), row.stripped_sha256, `module changed: ${path}`);
            modules.push({ backend, direction, path, bytes: bytes.length, sha256: hash(bytes),
                ...buildMetadata.backend_semantics?.[backend] });
        }
    }
    return {
        modules,
        module_directions: directions,
        sizes_sha256: hash(sizesBytes),
        build_metadata_sha256: await fileHash(join(directory, "metadata.json")),
        build_metadata: buildMetadata,
    };
}

async function emptyOutput(directory) {
    try {
        assert.equal((await readdir(directory)).length, 0, `output directory is not empty: ${directory}`);
    } catch (error) {
        if (error.code !== "ENOENT") throw error;
        await mkdir(directory, { recursive: true });
    }
}

function warmed(call, check, initialNs, samples = SAMPLES) {
    assert(Number.isFinite(initialNs) && initialNs > 0);
    const rows = [];
    let iterations = initialNs >= MIN_BATCH_NS ? 1 : Math.min(MAX_ITERATIONS, Math.max(1, Math.ceil(MIN_BATCH_NS * 1.25 / initialNs)));
    for (let sample = 0; sample < samples;) {
        assert(iterations < 0x80000000, "batch iteration limit exceeded");
        const value = call(iterations);
        check(value);
        assert(value.elapsedNs > 0);
        const phase = value.elapsedNs >= MIN_BATCH_NS ? "warm" : "calibration";
        rows.push({ phase, sample, iterations, elapsed_ns: value.elapsedNs, linear_memory_pages: value.memoryBytes / PAGE_BYTES });
        if (phase === "warm") sample++;
        else {
            assert(iterations < MAX_ITERATIONS, "maximum repeat count cannot reach minimum batch duration");
            const growth = Math.min(16, Math.max(2, Math.ceil(MIN_BATCH_NS * 1.25 / value.elapsedNs)));
            iterations = Math.min(MAX_ITERATIONS, iterations * growth);
        }
    }
    return rows;
}

async function selfTest(directory, backends = BACKENDS) {
    assert.equal(fnv1a64(new Uint8Array()), "cbf29ce484222325");
    assert.equal(fnv1a64(TEXT.encode("a")), "af63dc4c8601ec8c");
    assert.equal(skipReason("five8", 32), null);
    assert.equal(skipReason("five8", 64), null);
    assert(skipReason("five8", 33));
    assert(skipReason("bs58", 1048576));
    assert.equal(skipReason("bs58", 65536), null);
    assert.equal(skipReason("ntt-same-owned", MAX_BYTES, "decode", MAX_BYTES * 2), null);
    assert.equal(skipReason("ntt-same-cached", MAX_BYTES, "encode"), null);
    assert.equal(skipReason("matrix-forced", 4097, "decode", 5596), null);
    assert(skipReason("matrix-forced", 8192, "decode", 11188));
    assert(skipReason("matrix-forced", 8193, "encode"));
    assert(skipReason("matrix-forced", 0, "encode"));
    const ordered = Array.from({ length: 10 }, (_, index) => index);
    assert.deepEqual(shuffledJobs(ordered, 20261024), [0, 4, 8, 5, 7, 9, 1, 2, 3, 6]);
    assert.deepEqual([...shuffledJobs(ordered, 20261024)].sort((a, b) => a - b), ordered);
    assert.deepEqual(shuffledJobs(ordered, 20261024), shuffledJobs(ordered, 20261024));
    assert.notDeepEqual(shuffledJobs(ordered, 20261024), ordered);
    // These bytes come from the native resource worker's data() self-check.
    for (const [size, hexadecimal] of [[1, "f1"], [8, "37a0bb4ee07bcfec"], [32, "2fb463573fa33c32e02bb3684e7ad28de2917ada296159b310ca2d299bf64b8d"]]) {
        assert.equal(Buffer.from(dataFor(size)).toString("hex"), hexadecimal);
    }
    assert.equal(fnv1a64(dataFor(32)), "b95817c7fb38a311");
    const vectors = [[[], ""], [[0], "1"], [[0, 0], "11"], [[0, 1], "12"], [[1], "2"], [[57], "z"], [[58], "21"], [[255], "5Q"]];
    for (const [bytes, text] of vectors) {
        assert.equal(encodeReference(bytes), text);
        assert.deepEqual(decodeReference(text), Uint8Array.from(bytes));
    }
    for (const size of [0, 1, 32, 1024]) {
        const bytes = dataFor(size);
        assert.deepEqual(decodeReference(encodeReference(bytes)), bytes);
    }
    assert.throws(() => decodeReference("0"));
    const rows = warmed(iterations => ({ elapsedNs: iterations * 1000, memoryBytes: PAGE_BYTES }), () => {}, MIN_BATCH_NS * 10);
    assert.equal(rows.filter(row => row.phase === "warm").length, SAMPLES);
    assert(rows.some(row => row.phase === "calibration"));
    assert(rows.filter(row => row.phase === "warm").every(row => row.elapsed_ns >= MIN_BATCH_NS));
    const longRows = warmed(iterations => ({ elapsedNs: iterations * MIN_BATCH_NS, memoryBytes: PAGE_BYTES }), () => {}, MIN_BATCH_NS);
    assert(longRows.every(row => row.phase === "warm" && row.iterations === 1));
    assert.throws(() => warmed(() => ({ elapsedNs: 1, memoryBytes: PAGE_BYTES }), () => {}, MIN_BATCH_NS), /cannot reach minimum/);
    assert.equal(csv(["a,b", 'c"d', 1]), '"a,b","c""d",1\n');
    const inspected = await inspectModules(directory, backends);
    await assert.rejects(() => emptyOutput(directory), /not empty/);
    for (const module of inspected.modules.filter(module => module.direction === "both")) {
        const probe = await openProbe(module.path);
        probe.checkAllocator();
        let moduleVectors = module.backend === "five8"
            ? [32, 64].flatMap(size => [new Uint8Array(size), dataFor(size)].map(bytes => [bytes, encodeReference(bytes)]))
            : vectors;
        const controlSizes = module.backend === "matrix-forced" ? [32, 4097, 8192]
            : module.backend.startsWith("ntt-same-") ? [8192] : [];
        moduleVectors = moduleVectors.concat(controlSizes.map(size => {
            const data = dataFor(size);
            return [data, encodeReference(data)];
        }));
        for (const [bytes, text] of moduleVectors) {
            const data = Uint8Array.from(bytes);
            for (const [operation, input, want] of [["encode", data, TEXT.encode(text)], ["decode", TEXT.encode(text), data]]) {
                if (skipReason(module.backend, data.length, operation, TEXT.encode(text).length)) continue;
                const capacity = operation === "decode" && module.backend !== "empty" ? data.length : undefined;
                const value = probe.call(operation, input, 3, capacity);
                assert.equal(value.written, module.backend === "empty" ? input.length : want.length);
                assert.deepEqual(value.output, module.backend === "empty" ? new Uint8Array(input.length) : want);
            }
        }
        const capacity = module.backend === "five8" ? 32 : undefined;
        assert.equal(probe.callChecked("decode", Uint8Array.of(255), capacity).written, 0xffffffff);
        if (probe.decodePrevalidated) assert.throws(() => probe.call("decode", Uint8Array.of(255), 1, capacity), TypeError);
        if (module.backend !== "empty") assert.equal(probe.call("decode", TEXT.encode("0"), 1, capacity).written, 0xffffffff);
        if (module.backend === "five8") assert.equal(probe.call("encode", Uint8Array.of(1)).written, 0xffffffff);
    }
    console.error("Wasm timing driver self-check passed; no benchmark samples collected.");
    return inspected;
}

async function worker([modulePath, backend, operation, fixturePath, samples]) {
    samples = Number(samples);
    assert(Number.isSafeInteger(samples) && samples > 0 && samples % 2 === 1);
    const data = new Uint8Array(await readFile(`${fixturePath}.bin`));
    const encoded = new Uint8Array(await readFile(`${fixturePath}.txt`));
    const input = operation === "encode" ? data : encoded;
    const want = backend === "empty" ? new Uint8Array(input.length) : operation === "encode" ? encoded : data;
    const moduleSha256 = await fileHash(modulePath);
    const probe = await openProbe(modulePath);
    if (operation === "decode") assert(probe.decodePrevalidated, "new timing protocol requires the prevalidated UTF-8 repeat export");
    const capacity = operation === "decode" && backend !== "empty" ? data.length : undefined;
    const check = value => {
        assert.equal(value.written, want.length, `${backend}/${operation}: returned length`);
        assert.deepEqual(value.output, want, `${backend}/${operation}: output`);
        assert(Number.isInteger(value.memoryBytes / PAGE_BYTES));
    };
    const rows = [];
    for (const phase of ["cold", "second"]) {
        const value = probe.call(operation, input, 1, capacity);
        check(value);
        rows.push({ phase, sample: 0, iterations: 1, elapsed_ns: value.elapsedNs, linear_memory_pages: value.memoryBytes / PAGE_BYTES });
    }
    rows.push(...warmed(iterations => probe.call(operation, input, iterations, capacity), check, rows[1].elapsed_ns, samples));
    process.stdout.write(JSON.stringify({ timing_protocol: TIMING_PROTOCOL, module_sha256: moduleSha256, driver_sha256: await fileHash(SELF), helper_sha256: await fileHash(HELPER), decoded_bytes: data.length, input_bytes: input.length, encoded_bytes: encoded.length, data_sha256: hash(data), encoded_sha256: hash(encoded), rows }));
}

function runWorker(arguments_) {
    return new Promise((resolveResult, reject) => {
        const child = spawn(process.execPath, [...FLAGS, SELF, "--worker", ...arguments_], { stdio: ["ignore", "pipe", "pipe"] });
        let output = "";
        let errors = "";
        child.stdout.on("data", bytes => { output += bytes; });
        child.stderr.on("data", bytes => { errors += bytes; });
        child.on("error", reject);
        child.on("close", code => {
            if (code !== 0) reject(new Error(`worker exited ${code}: ${errors}`));
            else {
                try { resolveResult(JSON.parse(output)); } catch (error) { reject(error); }
            }
        });
    });
}

function median(values) {
    const sorted = [...values].sort((a, b) => a - b);
    assert(sorted.length % 2 === 1);
    return sorted[(sorted.length - 1) / 2];
}

async function main(directory, output, { backends, sizes, samples, repeats, seed }) {
    assert(directory && output, `usage: node ${FLAGS.join(" ")} scripts/wasm_tradeoffs.mjs MODULE_DIR EMPTY_OUTPUT_DIR`);
    assert(FLAGS.every(flag => process.execArgv.includes(flag)), `run Node with ${FLAGS.join(" ")}`);
    assert(backends.includes("auto"), "include auto backend to bind the fixture oracle module");
    directory = resolve(directory);
    output = resolve(output);
    await emptyOutput(output);
    const inspected = await selfTest(directory, backends);
    assert.equal(inspected.build_metadata.decode_timing_contract, "prevalidated-utf8-repeat-v1");
    assert.equal(inspected.build_metadata.repeat_timing_contract, "opaque-slice-lengths-v1");
    const driverSha256 = await fileHash(SELF);
    const helperSha256 = await fileHash(HELPER);
    const fixtureDirectory = join(output, "fixtures");
    await mkdir(fixtureDirectory);
    const oracle = await openProbe(join(directory, "auto-both-cdylib.wasm.stripped"));
    const fixtures = [];
    for (const size of sizes) {
        const data = dataFor(size);
        const encoded = oracle.call("encode", data).output;
        assert(encoded);
        if (size <= 1024) {
            const text = new TextDecoder("utf-8", { fatal: true }).decode(encoded);
            assert.equal(text, encodeReference(data));
            assert.deepEqual(decodeReference(text), data);
        }
        await writeFile(join(fixtureDirectory, `${size}.bin`), data, { flag: "wx" });
        await writeFile(join(fixtureDirectory, `${size}.txt`), encoded, { flag: "wx" });
        fixtures.push({ decoded_bytes: size, encoded_bytes: encoded.length, data_sha256: hash(data), encoded_sha256: hash(encoded), data_fnv1a64: fnv1a64(data), encoded_fnv1a64: fnv1a64(encoded) });
    }
    const fixturesBySize = new Map(fixtures.map(fixture => [fixture.decoded_bytes, fixture]));
    const jobs = shuffledJobs(backends.flatMap(backend => sizes.flatMap(decoded_bytes => ["encode", "decode"].flatMap(operation =>
        skipReason(backend, decoded_bytes, operation, fixturesBySize.get(decoded_bytes).encoded_bytes) ? []
            : Array.from({ length: repeats }, (_, repeat) => ({ backend, operation, decoded_bytes, repeat }))))), seed);
    const metadata = {
        schema: 3, timing_protocol: TIMING_PROTOCOL, decode_timing_contract: "prevalidated-utf8-repeat-v1",
        repeat_timing_contract: "opaque-slice-lengths-v1", primary_clock: "elapsed_ns",
        created_utc: new Date().toISOString(), node: process.version, versions: process.versions,
        worker_flags: FLAGS, parent_flags: process.execArgv, platform: os.platform(), release: os.release(), arch: os.arch(),
        cpu_model: os.cpus()[0]?.model, logical_cpus: os.cpus().length,
        driver_sha256: driverSha256, helper_sha256: helperSha256,
        modules_directory: directory, ...inspected, fixtures,
        sizes, backends, samples, repeats, minimum_batch_ns: MIN_BATCH_NS,
        backend_registry: backendRegistry(backends), jobs, job_order_seed: seed, job_order_algorithm: "xorshift32-fisher-yates-v1",
        sampling_policy: { kind: "fixed-warm-batches", warm_batches_per_process: samples, process_repeats: repeats,
            minimum_batch_clock: "elapsed_ns",
            maximum_iterations: MAX_ITERATIONS, calibration_growth_min: 2, calibration_growth_max: 16,
            long_call_policy: "one conversion per batch when second conversion reaches minimum batch duration; remaining calibrations recorded" },
        linear_memory_page_bytes: PAGE_BYTES, expected_cases: jobs.length,
        input_pattern: "xorshift64 seed decoded length + 20261023; XOR left 13, right 7, left 17 with u64 truncation after each step; low byte; first byte OR 1 when nonempty; matches native resource worker",
        caller_output_capacity: "decoded fixture length for every real decoder; input length * 2 + 32 for encoders and empty control",
        isolation: "one fresh Node process and Wasm instance per backend/operation/size/repeat; sequential workers",
        timer_scope: "backend-only Rust repeat export; UTF-8 validation of actual Wasm input memory, source/output allocation, copying and output checks are outside each timed interval",
        cold_scope: "first conversion after instantiation and caller buffer allocation; excludes process/module startup and caller buffer allocation",
        oracle: "auto Wasm encoder prepares every text fixture before timing; independent JS BigInt reference through 1024 bytes; all decode outputs checked against original bytes; all encode outputs checked against shared fixture",
        requires_native_fixture_crosscheck: true,
        limits: "Node/V8 on this host only; no browser claim, CPU affinity or thermal control; random-pattern valid inputs only; linear-memory pages include caller buffers/static data/allocator and cannot identify live heap or RSS; empty baseline is reported without subtraction",
    };
    await writeFile(join(output, "metadata.json"), JSON.stringify(metadata, null, 2) + "\n", { flag: "wx" });
    await writeFile(join(output, "skipped.csv"), csv(["backend", "operation", "decoded_bytes", "reason"])
        + backends.flatMap(backend => sizes.flatMap(size => ["encode", "decode"].flatMap(operation => {
            const reason = skipReason(backend, size, operation, fixturesBySize.get(size).encoded_bytes);
            return reason ? [csv([backend, operation, size, reason])] : [];
        }))).join(""), { flag: "wx" });
    const raw = await open(join(output, "samples.csv"), "wx");
    const summary = await open(join(output, "summary.csv"), "wx");
    await raw.write(csv(["backend", "operation", "decoded_bytes", "repeat", "input_bytes", "encoded_bytes", "module_bytes", "module_sha256", "phase", "sample", "iterations", "elapsed_ns", "ns_per_call", "linear_memory_pages"]));
    await summary.write(csv(["backend", "operation", "decoded_bytes", "repeat", "input_bytes", "encoded_bytes", "module_bytes", "module_sha256", "cold_ns", "second_ns", "warm_median_ns", "warm_min_ns", "warm_max_ns", "warm_mad_ns", "cold_pages", "second_pages", "warm_max_pages"]));
    let completed = 0;
    try {
        const modulesByBackend = new Map(inspected.modules.filter(module => module.direction === "both").map(module => [module.backend, module]));
        for (const { backend, operation, decoded_bytes: size, repeat } of jobs) {
            const module = modulesByBackend.get(backend);
            const result = await runWorker([module.path, module.backend, operation, join(fixtureDirectory, String(size)), String(samples)]);
            assert.equal(result.timing_protocol, TIMING_PROTOCOL);
            assert.equal(result.module_sha256, module.sha256);
            assert.equal(result.driver_sha256, driverSha256);
            assert.equal(result.helper_sha256, helperSha256);
            const fixture = fixturesBySize.get(size);
            assert.equal(result.decoded_bytes, size);
            assert.equal(result.data_sha256, fixture.data_sha256);
            assert.equal(result.encoded_sha256, fixture.encoded_sha256);
            const prefix = [module.backend, operation, size, repeat, result.input_bytes, result.encoded_bytes, module.bytes, module.sha256];
            for (const row of result.rows) {
                await raw.write(csv([...prefix, row.phase, row.sample, row.iterations, row.elapsed_ns, row.elapsed_ns / row.iterations, row.linear_memory_pages]));
            }
            const warm = result.rows.filter(row => row.phase === "warm");
            assert.equal(warm.length, samples);
            assert(warm.every(row => row.elapsed_ns >= MIN_BATCH_NS));
            const perCall = warm.map(row => row.elapsed_ns / row.iterations);
            const center = median(perCall);
            await summary.write(csv([...prefix, result.rows[0].elapsed_ns, result.rows[1].elapsed_ns, center, Math.min(...perCall), Math.max(...perCall), median(perCall.map(value => Math.abs(value - center))), result.rows[0].linear_memory_pages, result.rows[1].linear_memory_pages, Math.max(...warm.map(row => row.linear_memory_pages))]));
            console.error(`${++completed}/${metadata.expected_cases}: ${module.backend} ${operation} ${size} bytes repeat ${repeat}`);
        }
    } finally {
        await raw.close();
        await summary.close();
    }
    assert.equal(completed, metadata.expected_cases);
    assert.equal(await fileHash(SELF), driverSha256, "driver changed during run");
    assert.equal(await fileHash(HELPER), helperSha256, "helper changed during run");
    const checked = await inspectModules(directory, backends);
    assert.deepEqual(checked, inspected, "module inputs changed during run");
    for (const fixture of fixtures) {
        assert.equal(await fileHash(join(fixtureDirectory, `${fixture.decoded_bytes}.bin`)), fixture.data_sha256);
        assert.equal(await fileHash(join(fixtureDirectory, `${fixture.decoded_bytes}.txt`)), fixture.encoded_sha256);
    }
    const hashes = {};
    for (const name of ["metadata.json", "samples.csv", "summary.csv", "skipped.csv"]) hashes[name] = await fileHash(join(output, name));
    await writeFile(join(output, "complete.json"), JSON.stringify({ completed_cases: completed, completed_utc: new Date().toISOString(), sha256: hashes }, null, 2) + "\n", { flag: "wx" });
    console.error(`Completed ${completed} cases: ${output}`);
}

const args = process.argv.slice(2);
if (args[0] === "--worker") await worker(args.slice(1));
else {
    const options = { backends: BACKENDS, sizes: SIZES, samples: SAMPLES, repeats: 3, seed: 20261024 };
    const positional = [];
    let selfTestOnly = false;
    for (let index = 0; index < args.length; index++) {
        const argument = args[index];
        if (argument === "--self-test") selfTestOnly = true;
        else if (["--backends", "--sizes", "--samples", "--repeats", "--seed"].includes(argument)) {
            const value = args[++index];
            assert(value, `${argument} requires a value`);
            options[argument.slice(2)] = argument === "--backends" ? value.split(",")
                : argument === "--sizes" ? value.split(",").map(Number) : Number(value);
        } else {
            assert(!argument.startsWith("--"), `unknown option: ${argument}`);
            positional.push(argument);
        }
    }
    assert(options.backends.length && new Set(options.backends).size === options.backends.length);
    assert(options.backends.every(backend => ALL_BACKENDS.includes(backend)), "unknown backend");
    assert(options.sizes.length && new Set(options.sizes).size === options.sizes.length);
    assert(options.sizes.every(size => Number.isSafeInteger(size) && size >= 0 && size <= MAX_BYTES), `sizes must be decoded bytes in [0, ${MAX_BYTES}]`);
    assert(Number.isSafeInteger(options.samples) && options.samples > 0 && options.samples % 2 === 1, "samples must be a positive odd integer");
    assert(Number.isSafeInteger(options.repeats) && options.repeats > 0, "repeats must be a positive integer");
    assert(Number.isSafeInteger(options.seed) && options.seed > 0 && options.seed <= 0xffffffff, "seed must be a nonzero u32");
    if (selfTestOnly) {
        assert.equal(positional.length, 1, "--self-test requires MODULE_DIR");
        await selfTest(resolve(positional[0]), options.backends);
    } else {
        assert.equal(positional.length, 2, "pass MODULE_DIR and EMPTY_OUTPUT_DIR, or --self-test MODULE_DIR");
        await main(...positional, options);
    }
}
