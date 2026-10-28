"""Require actual mutation attempts, separately from libFuzzer callback counts."""

import argparse
import json
from pathlib import Path
import re
import sys


def require(condition, message):
    if not condition:
        raise ValueError(message)


def read_stats(text, seconds):
    def one(pattern):
        rows = list(re.finditer(pattern, text, re.MULTILINE))
        require(len(rows) == 1, 'missing or repeated final/initialization statistics')
        return rows[0]

    init = one(r'^#(\d+)[ \t]+INITED\b[^\n]*$')
    done = one(r'^#(\d+)[ \t]+DONE\b[^\n]*$')
    final = one(r'^stat::number_of_executed_units:[ \t]*(\d+)[ \t]*$')
    summary = one(r'^Done (\d+) runs in (\d+) second\(s\)[ \t]*$')
    require(init.start() < done.start() < summary.start() < final.start(),
            'unexpected initialization/final-statistics ordering')
    initial, callbacks = int(init[1]), int(done[1])
    runs, elapsed = map(int, summary.groups())
    # The pinned driver prints the entire corpus table between these delimiters.
    table = [line for line in text[summary.end():final.start()].splitlines() if line.strip()]
    rows = []
    for line in table:
        row = re.fullmatch(
            r'[ \t]*\[[ \t]*(\d+) [0-9a-f]{40}\] sz:[ \t]*\d+ runs:[ \t]*(\d+) succ:[ \t]*\d+ focus: [01][ \t]*',
            line,
        )
        require(row is not None, 'malformed corpus-statistics entry')
        rows.append(row.groups())
    candidates = list(re.finditer(r'^[ \t]*\[[^\n]*$', text, re.MULTILINE))
    require(all(summary.end() <= row.start() < final.start() for row in candidates),
            'corpus-statistics entry outside the final table')
    require([int(index) for index, _ in rows] == list(range(len(rows))),
            'missing, reordered or repeated corpus statistics')
    mutations = sum(int(count) for _, count in rows)
    require(mutations > 0, 'no post-initialization mutation attempts')
    require(callbacks == int(final[1]) == runs, 'inconsistent callback counts')
    require(initial > 0 and callbacks - initial >= mutations,
            'mutation count exceeds post-initialization callbacks')
    require(elapsed >= seconds, 'campaign ended before its time budget')
    return dict(mutation_attempts=mutations, callbacks=callbacks,
                initialization_callbacks=initial, reported_seconds=elapsed,
                corpus_entries=len(rows), counter='sum(InputInfo::NumExecutedMutations)',
                scope='post-initialization mutation attempts, not unique inputs or conversions')


def self_test():
    example = (
        '#2\tINITED cov: 10\n#9\tDONE cov: 11\nDone 9 runs in 181 second(s)\n'
        f'  [  0 {"0" * 40}] sz:     1 runs:     3 succ:     1 focus: 0\n'
        f'  [  1 {"1" * 40}] sz:     2 runs:     2 succ:     0 focus: 0\n'
        'stat::number_of_executed_units: 9\n'
    )
    require(read_stats(example, 180)['mutation_attempts'] == 5, 'positive fixture')
    bad_reports = [
        example.replace('runs:     3', 'runs:     0').replace('runs:     2', 'runs:     0'),
        example.replace('stat::number_of_executed_units: 9', ''),
        example.replace('#9\tDONE', '#9\tDONE\n#9\tDONE'),
        example.replace('second(s)', 'seconds'),
        example.replace('181 second', '179 second'),
        example.replace('runs:     3', 'runs:    30'),
        example.replace('runs:     2', 'runs:   bad'),
        example.replace('[  1 ', '[  2 '),
        example.replace('[  1 ', '[  0 '),
        example.replace('stat::number_of_executed_units: 9',
                        'malformed trailing entry\nstat::number_of_executed_units: 9'),
        example + f'  [  2 {"2" * 40}] sz:     1 runs:     1 succ:     0 focus: 0\n',
        example.replace('Done 9 runs in 181 second(s)\n', '')
            + 'Done 9 runs in 181 second(s)\n',
    ]
    for bad in bad_reports:
        try:
            read_stats(bad, 180)
        except ValueError:
            pass
        else:
            raise AssertionError('accepted invalid mutation statistics')
    print(f'mutation statistics self-test: pass (one positive, {len(bad_reports)} negative)')


def main():
    if sys.flags.optimize:
        raise RuntimeError('validation requires Python without -O')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('log', type=Path, nargs='?')
    parser.add_argument('--seconds', type=int, default=180)
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return
    require(args.log is not None and args.seconds > 0, 'log and positive budget required')
    print(json.dumps(read_stats(args.log.read_text(), args.seconds), indent=2))


if __name__ == '__main__':
    main()
