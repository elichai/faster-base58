# Selected final linked assembly

Static excerpts. Addresses are artifact-specific; instruction counts are not timing estimates.

## generic-baseline-default-auto: detector

`faster_base58::ntt::mul_with`

```asm
0x46bfd: movq %rdi, 0x18(%rsp)
0x46c02: jae 0x46c28 <faster_base58::ntt::mul_with+0x88>
0x46c04: movzbl 0x4d1d5(%rip), %eax     # 0x93de0 <faster_base58::cpu::detect_ifma::STORAGE (.0)>
0x46c0b: cmpb $0x1, %al
0x46c0d: je 0x473a3 <faster_base58::ntt::mul_with+0x803>
0x46c13: movzbl %al, %eax
```

## generic-baseline-default-auto: detector

`faster_base58::ntt::mul_with`

```asm
0x4737f: movq %r12, %r13
0x47382: movq %r8, %r12
0x47385: callq 0x5af40 <faster_base58::cpu::detect_ifma::init_get::init_inner>
0x4738a: movq %r12, %r8
0x4738d: movq %r13, %r12
0x47390: movq %r15, %r9
```

## generic-baseline-default-auto: threshold

`faster_base58::ntt::mul_with`

```asm
0x4739b: testb %al, %al
0x4739d: je 0x46c21 <faster_base58::ntt::mul_with+0x81>
0x473a3: movl $0x180, %eax            # imm = 0x180
0x473a8: subq $0x8, %rsp
0x473ac: movq %r14, %rsi
0x473af: movq 0x28(%rsp), %rdx
```

## generic-baseline-cache-auto: detector

`faster_base58::ntt::mul_with`

```asm
0x4097d: movq %rdi, 0x10(%rsp)
0x40982: jae 0x409bd <faster_base58::ntt::mul_with+0x9d>
0x40984: movq 0x54035(%rip), %rax     # 0x949c0 <std_detect::detect::cache::CACHE (.0)>
0x4098b: testq %rax, %rax
0x4098e: je 0x41109 <faster_base58::ntt::mul_with+0x7e9>
0x40994: testl $0x80000, %eax          # imm = 0x80000
```

## generic-baseline-cache-auto: detector

`faster_base58::ntt::mul_with`

```asm
0x40994: testl $0x80000, %eax          # imm = 0x80000
0x40999: je 0x409b6 <faster_base58::ntt::mul_with+0x96>
0x4099b: movq 0x5401e(%rip), %rax     # 0x949c0 <std_detect::detect::cache::CACHE (.0)>
0x409a2: testq %rax, %rax
0x409a5: je 0x4113b <faster_base58::ntt::mul_with+0x81b>
0x409ab: testl $0x4000000, %eax        # imm = 0x4000000
```

## generic-baseline-cache-auto: threshold

`faster_base58::ntt::mul_with`

```asm
0x4115d: testl $0x4000000, %eax        # imm = 0x4000000
0x41162: je 0x409b6 <faster_base58::ntt::mul_with+0x96>
0x41168: movl $0x180, %eax            # imm = 0x180
0x4116d: subq $0x8, %rsp
0x41171: movq %r14, %rsi
0x41174: movq 0x28(%rsp), %rdx
```

## baseline-default-auto: variable-step-division

`faster_base58::ntt::neon::forward`

```asm
0xe624: cmp x10, x8
0xe628: b.hi 0xe78c <faster_base58::ntt::neon::forward+0x1d4>
0xe62c: cbz x10, 0xe7a4 <faster_base58::ntt::neon::forward+0x1ec>
0xe630: udiv x14, x1, x10
0xe634: msub x10, x14, x10, x1
0xe638: cmp x10, #0x0
0xe63c: cinc x10, x14, ne
0xe640: cbz x10, 0xe604 <faster_base58::ntt::neon::forward+0x4c>
```
