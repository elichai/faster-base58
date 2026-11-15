"""Keep ISA replay, mutation, merge, publication and upload counts in agreement."""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parent.parent
WORKFLOW = (ROOT / ".github/workflows/fuzz-corpus.yml").read_text()


def job(name):
    return re.search(rf"^  {name}:\n(.*?)(?=^  [a-z_]+:|\Z)",
                     WORKFLOW, re.M | re.S).group(1)


def rows(name):
    result = set()
    for row in re.findall(r"^          - \{ (.*) \}$", job(name), re.M):
        fields = dict(re.findall(r"(runner|cpu|isa|label): ([\w.-]+)", row))
        result.add(tuple(fields[k] for k in ("runner", "cpu", "isa", "label")))
    return result


class IsaWorkflowTests(unittest.TestCase):
    def test_all_process_stages_select_the_same_isa_builds(self):
        x86 = {("ubuntu-26.04", "generic", isa, cache)
               for isa in ("host", "avx2", "sse41") for cache in ("default", "cache")}
        replay = x86 | {("ubuntu-26.04-arm", "generic", "host", "cache")}
        campaigns = replay | {("ubuntu-26.04-arm", "generic", "host", "default")}
        self.assertEqual(rows("fuzz_coverage"), replay)
        self.assertEqual(rows("candidate_smoke"), replay)
        self.assertEqual(rows("nightly"), campaigns)
        self.assertEqual(rows("minimize"), campaigns)
        for name in ("fuzz_coverage", "nightly", "minimize", "candidate_smoke"):
            self.assertIn("FASTER_BASE58_FUZZ_MAX_ISA: ${{ matrix.", job(name))

    def test_upload_count_and_artifact_identity(self):
        targets = re.search(r"fuzz_target: \[([^]]+)\]", job("fuzz_coverage"))[1].split(",")
        count = len(rows("fuzz_coverage")) * len(targets)
        config = (ROOT / "codecov.yml").read_text()
        total = int(re.search(r"after_n_builds: (\d+)", config)[1])
        fuzz = int(re.search(r"^  fuzz:\n.*?after_n_builds: (\d+)", config, re.M | re.S)[1])
        self.assertEqual(fuzz, count)
        self.assertEqual(total, count + 5)
        for name in ("fuzz_coverage", "nightly", "minimize"):
            self.assertRegex(job(name), r"name: fuzz-.*\$\{\{ matrix\.(?:row\.)?isa \}\}")
        self.assertIn("--field max_isa=${{ matrix.isa }}", job("nightly"))


if __name__ == "__main__":
    unittest.main()
