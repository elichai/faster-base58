"""Pure declaration/buffer-bound checks; never load ELF or run conversions."""
from collections import Counter,defaultdict
import unittest

import fixed_inode_v2 as observer


class ScheduleTests(unittest.TestCase):
    def test_study_covers_every_feature_cell_and_all_controls(self):
        cases = observer.study_cases()
        self.assertEqual(Counter(case['study'] for case in cases),dict(feature=232,negative=56))
        feature = {(case['cpu'],case['policy'],case['operation'],case['size'])
                   for case in cases if case['study']=='feature'}
        self.assertEqual(len(feature),232)
        self.assertEqual({row[-1] for row in feature},set(observer.SIZES))
        jobs = observer.schedule('study')
        self.assertEqual(len(jobs),2880)
        self.assertEqual(jobs,observer.schedule('study'))
        pairs = defaultdict(list)
        for job in jobs:
            pairs[job['case_index'],job['repeat']].append(job)
        self.assertEqual(len(pairs),1440)
        self.assertEqual(Counter(pair[0]['label'] for pair in pairs.values()),dict(alpha=720,bravo=720))
        for pair in pairs.values():
            self.assertEqual({job['label'] for job in pair},{'alpha','bravo'})
            self.assertEqual(len({job['image'] for job in pair}),2 if pair[0]['study']=='feature' else 1)

    def test_qualification_reaches_every_original_image(self):
        jobs = observer.schedule('qualify')
        counts = Counter(job['image'] for job in jobs)
        self.assertEqual(len(jobs),498)
        self.assertEqual(len(counts),17)
        self.assertEqual(counts['reference'],2)
        for image,count in counts.items():
            if image!='reference':
                self.assertEqual(count,58 if image.endswith('-auto') else 4)

    def test_validation_changes_only_the_label(self):
        jobs = observer.schedule('validation')
        self.assertEqual(len(jobs),10)
        self.assertEqual({job['image'] for job in jobs},{'reference'})
        self.assertEqual({job['size'] for job in jobs},{23})
        self.assertEqual({job['operation'] for job in jobs},{'decode'})
        self.assertEqual({len(job['label']) for job in jobs},{5})

    def test_all_buffer_residues_preserve_guard_spans(self):
        for base in range(128):
            for minimum,residue,extra in [(1,1,82),(3,3,84)]:
                offset = observer.offset(base,minimum,residue)
                self.assertGreaterEqual(offset,minimum)
                self.assertLess(offset,minimum+64)
                self.assertEqual((base+offset)%64,residue)
                self.assertGreaterEqual(extra-offset,17)


if __name__=='__main__':
    unittest.main()
