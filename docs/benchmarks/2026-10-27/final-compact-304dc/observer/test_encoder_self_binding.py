"""Pure narrow relocation predicates; never load an ELF or read process memory."""
import unittest

from fixed_inode_v2 import verify_encoder_self_binding


BASE = 0x10000000
OFFSET = 0x12340
POINTER = BASE+OFFSET


def entry(*, address=POINTER, base=BASE, offset=OFFSET, symbol='size_encode'):
    return dict(offset=0x8000,kind=6,symbol=symbol,
                actual=dict(address=address,module_base=base,module_offset=offset))


class EncoderSelfBindingTests(unittest.TestCase):
    def test_matching_self_entry_passes(self):
        result = verify_encoder_self_binding(BASE,OFFSET,POINTER,[entry()])
        self.assertEqual(result['status'],'verified')
        self.assertEqual(result['entries'],1)

    def test_wrong_address_rejected_even_with_matching_reported_offset(self):
        with self.assertRaisesRegex(RuntimeError,'own export pointer'):
            verify_encoder_self_binding(BASE,OFFSET,POINTER,[entry(address=POINTER+16)])

    def test_wrong_resolved_module_offset_rejected(self):
        with self.assertRaisesRegex(RuntimeError,'resolved module/offset'):
            verify_encoder_self_binding(BASE,OFFSET,POINTER,[entry(offset=OFFSET+16)])

    def test_wrong_own_export_binding_rejected(self):
        with self.assertRaisesRegex(RuntimeError,'own module offset'):
            verify_encoder_self_binding(BASE,OFFSET,POINTER+16,[entry()])

    def test_no_self_relocation_is_explicitly_not_applicable(self):
        result = verify_encoder_self_binding(BASE,OFFSET,POINTER,[])
        self.assertEqual(result['status'],'not-applicable')
        self.assertIn('No parsed',result['reason'])

    def test_unrelated_target_remains_record_only(self):
        diagnostic = entry(address=0,base=0,offset=0,symbol='calloc')
        result = verify_encoder_self_binding(BASE,OFFSET,POINTER,[diagnostic])
        self.assertEqual(result['status'],'not-applicable')
        self.assertEqual(result['entries'],0)


if __name__ == '__main__':
    unittest.main()
