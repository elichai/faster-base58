0000000000056290 <faster_base58::encode::batched::encode_parts>:
   56290:	55                   	push   %rbp
   56291:	41 57                	push   %r15
   56293:	41 56                	push   %r14
   56295:	41 55                	push   %r13
   56297:	41 54                	push   %r12
   56299:	53                   	push   %rbx
   5629a:	48 83 ec 58          	sub    $0x58,%rsp
   5629e:	48 89 fb             	mov    %rdi,%rbx
   562a1:	48 85 d2             	test   %rdx,%rdx
   562a4:	0f 84 3a 02 00 00    	je     564e4 <faster_base58::encode::batched::encode_parts+0x254>
   562aa:	0f b6 06             	movzbl (%rsi),%eax
   562ad:	49 89 d1             	mov    %rdx,%r9
   562b0:	f3 0f bd c0          	lzcnt  %eax,%eax
   562b4:	83 c0 e8             	add    $0xffffffe8,%eax
   562b7:	49 29 d8             	sub    %rbx,%r8
   562ba:	f6 d0                	not    %al
   562bc:	48 0f be c0          	movsbq %al,%rax
   562c0:	48 8d 14 d0          	lea    (%rax,%rdx,8),%rdx
   562c4:	48 b8 68 10 45 58 e7 	movabs $0x2bb378e758451068,%rax
   562cb:	78 b3 2b
   562ce:	c4 62 8b f6 f0       	mulx   %rax,%r14,%r14
   562d3:	b8 01 00 00 00       	mov    $0x1,%eax
   562d8:	0f 82 bc 02 00 00    	jb     5659a <faster_base58::encode::batched::encode_parts+0x30a>
   562de:	4d 39 f0             	cmp    %r14,%r8
   562e1:	0f 86 b3 02 00 00    	jbe    5659a <faster_base58::encode::batched::encode_parts+0x30a>
   562e7:	49 8d 46 01          	lea    0x1(%r14),%rax
   562eb:	48 89 4c 24 28       	mov    %rcx,0x28(%rsp)
   562f0:	4c 8d 3c 19          	lea    (%rcx,%rbx,1),%r15
   562f4:	48 8d 7c 24 38       	lea    0x38(%rsp),%rdi
   562f9:	4c 89 ca             	mov    %r9,%rdx
   562fc:	4c 89 44 24 30       	mov    %r8,0x30(%rsp)
   56301:	48 89 5c 24 20       	mov    %rbx,0x20(%rsp)
   56306:	c5 f8 57 c0          	vxorps %xmm0,%xmm0,%xmm0
   5630a:	89 c1                	mov    %eax,%ecx
   5630c:	83 e1 07             	and    $0x7,%ecx
   5630f:	48 c1 e8 03          	shr    $0x3,%rax
   56313:	4c 01 f9             	add    %r15,%rcx
   56316:	48 89 4c 24 38       	mov    %rcx,0x38(%rsp)
   5631b:	48 89 44 24 40       	mov    %rax,0x40(%rsp)
   56320:	c5 f8 11 44 24 48    	vmovups %xmm0,0x48(%rsp)
   56326:	e8 95 f5 ff ff       	call   558c0 <<faster_base58::encode::batched::Limbs>::feed>
   5632b:	48 8b 4c 24 50       	mov    0x50(%rsp),%rcx
   56330:	48 b8 b5 6a 01 1a ad 	movabs $0x68b2c7ad1a016ab5,%rax
   56337:	c7 b2 68
   5633a:	48 8b 6c 24 48       	mov    0x48(%rsp),%rbp
   5633f:	48 89 ca             	mov    %rcx,%rdx
   56342:	c4 e2 fb f6 c0       	mulx   %rax,%rax,%rax
   56347:	48 c1 e8 1c          	shr    $0x1c,%rax
   5634b:	89 c2                	mov    %eax,%edx
   5634d:	48 69 fa f5 04 e2 5e 	imul   $0x5ee204f5,%rdx,%rdi
   56354:	48 c1 ef 36          	shr    $0x36,%rdi
   56358:	3d a0 35 1f 27       	cmp    $0x271f35a0,%eax
   5635d:	0f 83 5a 04 00 00    	jae    567bd <faster_base58::encode::batched::encode_parts+0x52d>
   56363:	48 69 d0 a0 35 1f 27 	imul   $0x271f35a0,%rax,%rdx
   5636a:	89 c6                	mov    %eax,%esi
   5636c:	48 8d 1d b7 44 fb ff 	lea    -0x4bb49(%rip),%rbx        # a82a <core::unicode::unicode_data::cf::SHORT_OFFSET_RUNS+0x582>
   56373:	48 29 d1             	sub    %rdx,%rcx
   56376:	69 d7 10 ad ac 00    	imul   $0xacad10,%edi,%edx
   5637c:	41 89 cb             	mov    %ecx,%r11d
   5637f:	29 d6                	sub    %edx,%esi
   56381:	48 8d 15 68 44 fb ff 	lea    -0x4bb98(%rip),%rdx        # a7f0 <core::unicode::unicode_data::cf::SHORT_OFFSET_RUNS+0x548>
   56388:	0f b6 14 17          	movzbl (%rdi,%rdx,1),%edx
   5638c:	48 bf 00 00 90 24 48 	movabs $0x137b4824900000,%rdi
   56393:	7b 13 00
   56396:	88 54 24 09          	mov    %dl,0x9(%rsp)
   5639a:	89 f2                	mov    %esi,%edx
   5639c:	c4 e2 eb f6 d7       	mulx   %rdi,%rdx,%rdx
   563a1:	69 fa 24 0d 00 00    	imul   $0xd24,%edx,%edi
   563a7:	44 0f b6 44 53 01    	movzbl 0x1(%rbx,%rdx,2),%r8d
   563ad:	44 0f b6 14 53       	movzbl (%rbx,%rdx,2),%r10d
   563b2:	29 fe                	sub    %edi,%esi
   563b4:	0f b6 3c 73          	movzbl (%rbx,%rsi,2),%edi
   563b8:	0f b6 54 73 01       	movzbl 0x1(%rbx,%rsi,2),%edx
   563bd:	89 fe                	mov    %edi,%esi
   563bf:	89 54 24 18          	mov    %edx,0x18(%rsp)
   563c3:	c1 e2 18             	shl    $0x18,%edx
   563c6:	c1 e6 10             	shl    $0x10,%esi
   563c9:	89 7c 24 1c          	mov    %edi,0x1c(%rsp)
   563cd:	09 d6                	or     %edx,%esi
   563cf:	44 89 c2             	mov    %r8d,%edx
   563d2:	c1 e2 08             	shl    $0x8,%edx
   563d5:	44 09 d2             	or     %r10d,%edx
   563d8:	09 f2                	or     %esi,%edx
   563da:	48 69 f1 f5 04 e2 5e 	imul   $0x5ee204f5,%rcx,%rsi
   563e1:	48 c1 ee 36          	shr    $0x36,%rsi
   563e5:	44 69 ce 10 ad ac 00 	imul   $0xacad10,%esi,%r9d
   563ec:	45 29 cb             	sub    %r9d,%r11d
   563ef:	4d 69 cb 83 b4 37 01 	imul   $0x137b483,%r11,%r9
   563f6:	49 c1 e9 24          	shr    $0x24,%r9
   563fa:	45 69 e1 24 0d 00 00 	imul   $0xd24,%r9d,%r12d
   56401:	45 29 e3             	sub    %r12d,%r11d
   56404:	42 0f b6 3c 5b       	movzbl (%rbx,%r11,2),%edi
   56409:	46 0f b6 64 5b 01    	movzbl 0x1(%rbx,%r11,2),%r12d
   5640f:	46 0f b6 5c 4b 01    	movzbl 0x1(%rbx,%r9,2),%r11d
   56415:	46 0f b6 0c 4b       	movzbl (%rbx,%r9,2),%r9d
   5641a:	41 89 fd             	mov    %edi,%r13d
   5641d:	44 89 64 24 14       	mov    %r12d,0x14(%rsp)
   56422:	41 c1 e4 18          	shl    $0x18,%r12d
   56426:	41 c1 e5 10          	shl    $0x10,%r13d
   5642a:	45 09 e5             	or     %r12d,%r13d
   5642d:	45 89 dc             	mov    %r11d,%r12d
   56430:	41 c1 e4 08          	shl    $0x8,%r12d
   56434:	45 09 cc             	or     %r9d,%r12d
   56437:	45 09 ec             	or     %r13d,%r12d
   5643a:	44 0f b6 6c 24 09    	movzbl 0x9(%rsp),%r13d
   56440:	44 88 6c 24 0a       	mov    %r13b,0xa(%rsp)
   56445:	89 54 24 0b          	mov    %edx,0xb(%rsp)
   56449:	48 8d 15 a0 43 fb ff 	lea    -0x4bc60(%rip),%rdx        # a7f0 <core::unicode::unicode_data::cf::SHORT_OFFSET_RUNS+0x548>
   56450:	0f b6 14 16          	movzbl (%rsi,%rdx,1),%edx
   56454:	88 54 24 0f          	mov    %dl,0xf(%rsp)
   56458:	48 8b 54 24 30       	mov    0x30(%rsp),%rdx
   5645d:	44 89 64 24 10       	mov    %r12d,0x10(%rsp)
   56462:	3d 0f ad ac 00       	cmp    $0xacad0f,%eax
   56467:	b8 01 00 00 00       	mov    $0x1,%eax
   5646c:	0f 87 9a 00 00 00    	ja     5650c <faster_base58::encode::batched::encode_parts+0x27c>
   56472:	41 80 fa 31          	cmp    $0x31,%r10b
   56476:	0f 85 9d 00 00 00    	jne    56519 <faster_base58::encode::batched::encode_parts+0x289>
   5647c:	41 80 f8 31          	cmp    $0x31,%r8b
   56480:	0f 85 a0 00 00 00    	jne    56526 <faster_base58::encode::batched::encode_parts+0x296>
   56486:	80 7c 24 1c 31       	cmpb   $0x31,0x1c(%rsp)
   5648b:	0f 85 a2 00 00 00    	jne    56533 <faster_base58::encode::batched::encode_parts+0x2a3>
   56491:	80 7c 24 18 31       	cmpb   $0x31,0x18(%rsp)
   56496:	0f 85 a4 00 00 00    	jne    56540 <faster_base58::encode::batched::encode_parts+0x2b0>
   5649c:	48 81 f9 0f ad ac 00 	cmp    $0xacad0f,%rcx
   564a3:	0f 87 a4 00 00 00    	ja     5654d <faster_base58::encode::batched::encode_parts+0x2bd>
   564a9:	41 80 f9 31          	cmp    $0x31,%r9b
   564ad:	0f 85 a7 00 00 00    	jne    5655a <faster_base58::encode::batched::encode_parts+0x2ca>
   564b3:	41 80 fb 31          	cmp    $0x31,%r11b
   564b7:	0f 85 aa 00 00 00    	jne    56567 <faster_base58::encode::batched::encode_parts+0x2d7>
   564bd:	40 80 ff 31          	cmp    $0x31,%dil
   564c1:	0f 85 ad 00 00 00    	jne    56574 <faster_base58::encode::batched::encode_parts+0x2e4>
   564c7:	45 31 e4             	xor    %r12d,%r12d
   564ca:	80 7c 24 14 31       	cmpb   $0x31,0x14(%rsp)
   564cf:	48 8d 74 24 0a       	lea    0xa(%rsp),%rsi
   564d4:	41 0f 95 c4          	setne  %r12b
   564d8:	4c 29 e6             	sub    %r12,%rsi
   564db:	48 83 c6 0a          	add    $0xa,%rsi
   564df:	e9 9b 00 00 00       	jmp    5657f <faster_base58::encode::batched::encode_parts+0x2ef>
   564e4:	49 39 d8             	cmp    %rbx,%r8
   564e7:	73 0a                	jae    564f3 <faster_base58::encode::batched::encode_parts+0x263>
   564e9:	b8 01 00 00 00       	mov    $0x1,%eax
   564ee:	e9 51 02 00 00       	jmp    56744 <faster_base58::encode::batched::encode_parts+0x4b4>
   564f3:	48 8b 05 b6 8b 03 00 	mov    0x38bb6(%rip),%rax        # 8f0b0 <memset@GLIBC_2.2.5>
   564fa:	be 31 00 00 00       	mov    $0x31,%esi
   564ff:	48 89 cf             	mov    %rcx,%rdi
   56502:	48 89 da             	mov    %rbx,%rdx
   56505:	ff d0                	call   *%rax
   56507:	e9 36 02 00 00       	jmp    56742 <faster_base58::encode::batched::encode_parts+0x4b2>
   5650c:	41 bc 0a 00 00 00    	mov    $0xa,%r12d
   56512:	48 8d 74 24 0a       	lea    0xa(%rsp),%rsi
   56517:	eb 66                	jmp    5657f <faster_base58::encode::batched::encode_parts+0x2ef>
   56519:	48 8d 74 24 0b       	lea    0xb(%rsp),%rsi
   5651e:	41 bc 09 00 00 00    	mov    $0x9,%r12d
   56524:	eb 59                	jmp    5657f <faster_base58::encode::batched::encode_parts+0x2ef>
   56526:	48 8d 74 24 0c       	lea    0xc(%rsp),%rsi
   5652b:	41 bc 08 00 00 00    	mov    $0x8,%r12d
   56531:	eb 4c                	jmp    5657f <faster_base58::encode::batched::encode_parts+0x2ef>
   56533:	48 8d 74 24 0d       	lea    0xd(%rsp),%rsi
   56538:	41 bc 07 00 00 00    	mov    $0x7,%r12d
   5653e:	eb 3f                	jmp    5657f <faster_base58::encode::batched::encode_parts+0x2ef>
   56540:	48 8d 74 24 0e       	lea    0xe(%rsp),%rsi
   56545:	41 bc 06 00 00 00    	mov    $0x6,%r12d
   5654b:	eb 32                	jmp    5657f <faster_base58::encode::batched::encode_parts+0x2ef>
   5654d:	48 8d 74 24 0f       	lea    0xf(%rsp),%rsi
   56552:	41 bc 05 00 00 00    	mov    $0x5,%r12d
   56558:	eb 25                	jmp    5657f <faster_base58::encode::batched::encode_parts+0x2ef>
   5655a:	48 8d 74 24 10       	lea    0x10(%rsp),%rsi
   5655f:	41 bc 04 00 00 00    	mov    $0x4,%r12d
   56565:	eb 18                	jmp    5657f <faster_base58::encode::batched::encode_parts+0x2ef>
   56567:	48 8d 74 24 11       	lea    0x11(%rsp),%rsi
   5656c:	41 bc 03 00 00 00    	mov    $0x3,%r12d
   56572:	eb 0b                	jmp    5657f <faster_base58::encode::batched::encode_parts+0x2ef>
   56574:	48 8d 74 24 12       	lea    0x12(%rsp),%rsi
   56579:	41 bc 02 00 00 00    	mov    $0x2,%r12d
   5657f:	48 8d 4c ad 00       	lea    0x0(%rbp,%rbp,4),%rcx
   56584:	4d 8d 2c 4c          	lea    (%r12,%rcx,2),%r13
   56588:	49 8d 4e 03          	lea    0x3(%r14),%rcx
   5658c:	49 39 cd             	cmp    %rcx,%r13
   5658f:	0f 87 e8 01 00 00    	ja     5677d <faster_base58::encode::batched::encode_parts+0x4ed>
   56595:	49 39 d5             	cmp    %rdx,%r13
   56598:	76 05                	jbe    5659f <faster_base58::encode::batched::encode_parts+0x30f>
   5659a:	e9 a5 01 00 00       	jmp    56744 <faster_base58::encode::batched::encode_parts+0x4b4>
   5659f:	4d 39 ec             	cmp    %r13,%r12
   565a2:	0f 87 ed 01 00 00    	ja     56795 <faster_base58::encode::batched::encode_parts+0x505>
   565a8:	48 8b 05 09 8b 03 00 	mov    0x38b09(%rip),%rax        # 8f0b8 <memcpy@GLIBC_2.14>
   565af:	4c 89 ff             	mov    %r15,%rdi
   565b2:	4c 89 e2             	mov    %r12,%rdx
   565b5:	ff d0                	call   *%rax
   565b7:	48 8b 7c 24 28       	mov    0x28(%rsp),%rdi
   565bc:	48 85 ed             	test   %rbp,%rbp
   565bf:	0f 84 64 01 00 00    	je     56729 <faster_base58::encode::batched::encode_parts+0x499>
   565c5:	48 8d 04 ed 00 00 00 	lea    0x0(,%rbp,8),%rax
   565cc:	00
   565cd:	49 29 c6             	sub    %rax,%r14
   565d0:	49 83 c6 09          	add    $0x9,%r14
   565d4:	66 66 66 2e 0f 1f 84 	data16 data16 cs nopw 0x0(%rax,%rax,1)
   565db:	00 00 00 00 00
   565e0:	49 8d 46 f8          	lea    -0x8(%r14),%rax
   565e4:	48 83 f8 f7          	cmp    $0xfffffffffffffff7,%rax
   565e8:	0f 87 68 01 00 00    	ja     56756 <faster_base58::encode::batched::encode_parts+0x4c6>
   565ee:	4d 39 ee             	cmp    %r13,%r14
   565f1:	0f 87 5f 01 00 00    	ja     56756 <faster_base58::encode::batched::encode_parts+0x4c6>
   565f7:	49 8d 74 24 0a       	lea    0xa(%r12),%rsi
   565fc:	49 83 fc f5          	cmp    $0xfffffffffffffff5,%r12
   56600:	0f 87 65 01 00 00    	ja     5676b <faster_base58::encode::batched::encode_parts+0x4db>
   56606:	4c 39 ee             	cmp    %r13,%rsi
   56609:	0f 87 5c 01 00 00    	ja     5676b <faster_base58::encode::batched::encode_parts+0x4db>
   5660f:	4b 8b 44 37 f8       	mov    -0x8(%r15,%r14,1),%rax
   56614:	48 b9 b5 6a 01 1a ad 	movabs $0x68b2c7ad1a016ab5,%rcx
   5661b:	c7 b2 68
   5661e:	48 89 c2             	mov    %rax,%rdx
   56621:	c4 62 bb f6 c1       	mulx   %rcx,%r8,%r8
   56626:	49 c1 e8 1c          	shr    $0x1c,%r8
   5662a:	44 89 c1             	mov    %r8d,%ecx
   5662d:	48 69 c9 f5 04 e2 5e 	imul   $0x5ee204f5,%rcx,%rcx
   56634:	48 c1 e9 36          	shr    $0x36,%rcx
   56638:	41 81 f8 a0 35 1f 27 	cmp    $0x271f35a0,%r8d
   5663f:	0f 83 64 01 00 00    	jae    567a9 <faster_base58::encode::batched::encode_parts+0x519>
   56645:	44 69 c9 10 ad ac 00 	imul   $0xacad10,%ecx,%r9d
   5664c:	49 69 d0 a0 35 1f 27 	imul   $0x271f35a0,%r8,%rdx
   56653:	4c 8d 1d 96 41 fb ff 	lea    -0x4be6a(%rip),%r11        # a7f0 <core::unicode::unicode_data::cf::SHORT_OFFSET_RUNS+0x548>
   5665a:	48 ff cd             	dec    %rbp
   5665d:	49 83 c6 08          	add    $0x8,%r14
   56661:	42 0f b6 0c 19       	movzbl (%rcx,%r11,1),%ecx
   56666:	45 29 c8             	sub    %r9d,%r8d
   56669:	49 b9 00 00 90 24 48 	movabs $0x137b4824900000,%r9
   56670:	7b 13 00
   56673:	48 29 d0             	sub    %rdx,%rax
   56676:	44 89 c2             	mov    %r8d,%edx
   56679:	c4 42 b3 f6 c9       	mulx   %r9,%r9,%r9
   5667e:	43 88 0c 27          	mov    %cl,(%r15,%r12,1)
   56682:	41 69 d1 24 0d 00 00 	imul   $0xd24,%r9d,%edx
   56689:	41 29 d0             	sub    %edx,%r8d
   5668c:	42 0f b6 54 4b 01    	movzbl 0x1(%rbx,%r9,2),%edx
   56692:	46 0f b6 0c 4b       	movzbl (%rbx,%r9,2),%r9d
   56697:	46 0f b6 14 43       	movzbl (%rbx,%r8,2),%r10d
   5669c:	46 0f b6 44 43 01    	movzbl 0x1(%rbx,%r8,2),%r8d
   566a2:	c1 e2 08             	shl    $0x8,%edx
   566a5:	41 c1 e0 18          	shl    $0x18,%r8d
   566a9:	41 c1 e2 10          	shl    $0x10,%r10d
   566ad:	45 09 c2             	or     %r8d,%r10d
   566b0:	44 09 d2             	or     %r10d,%edx
   566b3:	44 09 ca             	or     %r9d,%edx
   566b6:	4c 69 c8 f5 04 e2 5e 	imul   $0x5ee204f5,%rax,%r9
   566bd:	43 89 54 27 01       	mov    %edx,0x1(%r15,%r12,1)
   566c2:	49 c1 e9 36          	shr    $0x36,%r9
   566c6:	47 0f b6 04 19       	movzbl (%r9,%r11,1),%r8d
   566cb:	45 69 c9 10 ad ac 00 	imul   $0xacad10,%r9d,%r9d
   566d2:	44 29 c8             	sub    %r9d,%eax
   566d5:	4c 69 c8 83 b4 37 01 	imul   $0x137b483,%rax,%r9
   566dc:	47 88 44 27 05       	mov    %r8b,0x5(%r15,%r12,1)
   566e1:	49 c1 e9 24          	shr    $0x24,%r9
   566e5:	45 69 d1 24 0d 00 00 	imul   $0xd24,%r9d,%r10d
   566ec:	44 29 d0             	sub    %r10d,%eax
   566ef:	46 0f b6 54 4b 01    	movzbl 0x1(%rbx,%r9,2),%r10d
   566f5:	46 0f b6 0c 4b       	movzbl (%rbx,%r9,2),%r9d
   566fa:	44 0f b6 1c 43       	movzbl (%rbx,%rax,2),%r11d
   566ff:	0f b6 44 43 01       	movzbl 0x1(%rbx,%rax,2),%eax
   56704:	41 c1 e2 08          	shl    $0x8,%r10d
   56708:	c1 e0 18             	shl    $0x18,%eax
   5670b:	41 c1 e3 10          	shl    $0x10,%r11d
   5670f:	41 09 c3             	or     %eax,%r11d
   56712:	45 09 da             	or     %r11d,%r10d
   56715:	45 09 ca             	or     %r9d,%r10d
   56718:	47 89 54 27 06       	mov    %r10d,0x6(%r15,%r12,1)
   5671d:	49 89 f4             	mov    %rsi,%r12
   56720:	48 85 ed             	test   %rbp,%rbp
   56723:	0f 85 b7 fe ff ff    	jne    565e0 <faster_base58::encode::batched::encode_parts+0x350>
   56729:	48 8b 5c 24 20       	mov    0x20(%rsp),%rbx
   5672e:	48 8b 05 7b 89 03 00 	mov    0x3897b(%rip),%rax        # 8f0b0 <memset@GLIBC_2.2.5>
   56735:	be 31 00 00 00       	mov    $0x31,%esi
   5673a:	48 89 da             	mov    %rbx,%rdx
   5673d:	ff d0                	call   *%rax
   5673f:	4c 01 eb             	add    %r13,%rbx
   56742:	31 c0                	xor    %eax,%eax
   56744:	48 89 da             	mov    %rbx,%rdx
   56747:	48 83 c4 58          	add    $0x58,%rsp
   5674b:	5b                   	pop    %rbx
   5674c:	41 5c                	pop    %r12
   5674e:	41 5d                	pop    %r13
   56750:	41 5e                	pop    %r14
   56752:	41 5f                	pop    %r15
   56754:	5d                   	pop    %rbp
   56755:	c3                   	ret
   56756:	48 8d 0d db 70 03 00 	lea    0x370db(%rip),%rcx        # 8d838 <__frame_dummy_init_array_entry+0x12e8>
   5675d:	48 89 c7             	mov    %rax,%rdi
   56760:	4c 89 f6             	mov    %r14,%rsi
   56763:	4c 89 ea             	mov    %r13,%rdx
   56766:	e8 d5 5f fc ff       	call   1c740 <core::slice::index::slice_index_fail>
   5676b:	48 8d 0d ae 70 03 00 	lea    0x370ae(%rip),%rcx        # 8d820 <__frame_dummy_init_array_entry+0x12d0>
   56772:	4c 89 e7             	mov    %r12,%rdi
   56775:	4c 89 ea             	mov    %r13,%rdx
   56778:	e8 c3 5f fc ff       	call   1c740 <core::slice::index::slice_index_fail>
   5677d:	48 8d 3d e2 3e fb ff 	lea    -0x4c11e(%rip),%rdi        # a666 <core::unicode::unicode_data::cf::SHORT_OFFSET_RUNS+0x3be>
   56784:	48 8d 15 7d 70 03 00 	lea    0x3707d(%rip),%rdx        # 8d808 <__frame_dummy_init_array_entry+0x12b8>
   5678b:	be 2b 00 00 00       	mov    $0x2b,%esi
   56790:	e8 db 41 fc ff       	call   1a970 <core::panicking::panic_fmt>
   56795:	48 8d 0d b4 70 03 00 	lea    0x370b4(%rip),%rcx        # 8d850 <__frame_dummy_init_array_entry+0x1300>
   5679c:	31 ff                	xor    %edi,%edi
   5679e:	4c 89 e6             	mov    %r12,%rsi
   567a1:	4c 89 ea             	mov    %r13,%rdx
   567a4:	e8 97 5f fc ff       	call   1c740 <core::slice::index::slice_index_fail>
   567a9:	48 8d 15 b8 73 03 00 	lea    0x373b8(%rip),%rdx        # 8db68 <__frame_dummy_init_array_entry+0x1618>
   567b0:	be 3a 00 00 00       	mov    $0x3a,%esi
   567b5:	48 89 cf             	mov    %rcx,%rdi
   567b8:	e8 8f 60 fc ff       	call   1c84c <core::panicking::panic_bounds_check>
   567bd:	48 8d 15 a4 73 03 00 	lea    0x373a4(%rip),%rdx        # 8db68 <__frame_dummy_init_array_entry+0x1618>
   567c4:	be 3a 00 00 00       	mov    $0x3a,%esi
   567c9:	e8 7e 60 fc ff       	call   1c84c <core::panicking::panic_bounds_check>
   567ce:	cc                   	int3
   567cf:	cc                   	int3
