#!/usr/bin/env python3
"""Review2 final-source observer: one inode, narrow encoder self-binding check."""
import argparse
import csv
import ctypes
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import random
import runpy
import shutil
import statistics
import subprocess
import sys
import time
import tomllib

from fixed_file import Scratch, padded_image, require, sha
from inspect_loaded_calls import Loaded

FINAL_SOURCE = '304dc4c5dbf18be9c6e1eab7a62b5dabb98022ff6532c1e5ba4fe5e6df000add'
REFERENCE_SHA = '3c5fc6b56da3d2814e008d4304d3bf7449cc8092d1495d7f3c4d6532abae60af'
SIZES = [0,1,8,16,21,22,23,31,32,33,63,64,65,128,256,512,1024,2048,4096,8192,
         16384,32768,65536,131072,262144,524288,1048576,2097152,4194304]
FLOOR = 100_000_000
CAP = 1_000_000_000
SAMPLES = 3
REPEATS = 5
SEED = 0
DEADLINE = datetime.fromisoformat('2026-10-28T06:29:14.717936+00:00')
ENV_KEYS = ['PATH','PYTHONPATH','PYTHONHOME','PYTHONHASHSEED','PYTHONMALLOC','PYTHONDEVMODE',
            'PYTHONOPTIMIZE','PYTHONNOUSERSITE','PYTHONWARNINGS','LD_LIBRARY_PATH','LD_PRELOAD',
            'LD_AUDIT','MALLOC_ARENA_MAX','GLIBC_TUNABLES','LANG','LC_ALL','TZ',
            'RUSTFLAGS','RUSTUP_TOOLCHAIN','OMP_NUM_THREADS','OPENBLAS_NUM_THREADS']
PARTIAL = {}


def digest(path):
    return sha(Path(path).read_bytes())


def read(path):
    return json.loads(Path(path).read_text())


def create(path, value):
    with Path(path).open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def environment():
    return {key:os.environ.get(key) for key in ENV_KEYS}


def sources():
    return {name:digest(Path(__file__).with_name(name)) for name in
            ['fixed_inode_v2.py','fixed_file.py','inspect_loaded_calls.py','process_supervisor.py','PROTOCOL.md']}


def study_cases():
    cases = []
    for cpu in ['generic','native']:
        for policy in ['default','cache']:
            for operation in ['encode','decode']:
                for size in SIZES:
                    cases.append(dict(study='feature',cpu=cpu,policy=policy,backend='auto',operation=operation,size=size))
                for backend, sizes in [('auto',[0,8,23,4096,4194304]),('empty',[0,23])]:
                    for size in sizes:
                        cases.append(dict(study='negative',cpu=cpu,policy=policy,backend=backend,operation=operation,size=size))
    require(len(cases)==288, 'Wrong predeclared case count')
    random.Random(SEED).shuffle(cases)
    return cases


def schedule(phase):
    if phase=='qualify':
        result = []
        for cpu in ['generic','native']:
            for policy in ['default','cache']:
                for variant,label in [('wide','alpha'),('slim','bravo')]:
                    for backend in ['auto','empty']:
                        for operation in ['encode','decode']:
                            for size in SIZES if backend=='auto' else [0,23]:
                                result.append(dict(study='qualification',cpu=cpu,policy=policy,backend=backend,
                                    operation=operation,size=size,case_index=len(result),repeat=0,label=label,
                                    image='-'.join([cpu,policy,variant,backend])))
        for label in ['alpha','bravo']:
            result.append(dict(study='qualification',cpu='native',policy='cache',backend='auto',
                operation='decode',size=23,case_index=len(result),repeat=0,label=label,image='reference'))
        require(len(result)==498, 'Wrong untimed qualification count')
        return result
    if phase=='validation':
        cases = [dict(study='known-control',cpu='native',policy='cache',backend='auto',operation='decode',size=23)]
        repeats = REPEATS
    else:
        cases = study_cases()
        repeats = REPEATS
    result = []
    for case_index,case in enumerate(cases):
        for repeat in range(repeats):
            labels = ['alpha','bravo']
            if (case_index+repeat)%2:
                labels.reverse()
            for label in labels:
                variant = 'slim' if case['study']=='feature' and label=='bravo' else 'wide'
                image = 'reference' if case['study']=='known-control' else '-'.join([case['cpu'],case['policy'],variant,case['backend']])
                result.append(dict(**case,case_index=case_index,repeat=repeat,label=label,image=image))
    require(len(result)=={'validation':10,'study':2880}[phase], 'Wrong predeclared process count')
    return result


def offset(base, minimum, residue):
    return minimum+(residue-base-minimum)%64


def mutable_state(library):
    result = []
    for name,symbol in library.elf.symbols.items():
        if not any(part in name for part in ['faster_base58','std_detect','cpufeatures']):
            continue
        if not any(part in name.lower() for part in ['storage','cache','table']):
            continue
        index = symbol['index']
        if symbol['info']&15!=1 or not (0<index<len(library.elf.sections)):
            continue
        section = library.elf.sections[index]
        if section[2]&3!=3:
            continue
        data = ctypes.string_at(library.base+symbol['address'],symbol['size'])
        result.append(dict(name=name,offset=symbol['address'],bytes=len(data),sha256=sha(data),prefix_hex=data[:64].hex()))
    return result


def verify_encoder_self_binding(base, symbol_offset, export_pointer, targets):
    """Check only parsed size_encode GOT entries against this validated export.

    Other relocation targets and writable objects remain record-only diagnostics.
    This pure predicate does not infer a complete dynamic-call graph.
    """
    require(export_pointer==base+symbol_offset, 'Encoder export differs from its own module offset')
    entries = [row for row in targets if row['symbol']=='size_encode']
    for row in entries:
        actual = row['actual']
        require(row['kind'] in [6,7], 'Unsupported encoder self-binding relocation')
        require(actual.get('address')==export_pointer, 'Encoder GOT target differs from its own export pointer')
        require(actual.get('module_base')==base and actual.get('module_offset')==symbol_offset,
                'Encoder GOT target differs from its own resolved module/offset')
    return dict(status='verified' if entries else 'not-applicable',entries=len(entries),
        export_pointer=export_pointer,export_module_offset=symbol_offset,relocations=entries,
        reason='All parsed size_encode GLOB_DAT/JUMP_SLOT targets equal the validated own export.' if entries else
               'No parsed size_encode GLOB_DAT/JUMP_SLOT relocation exists. Direct/inlined code is covered by mapped .text checks; no general call-target verification is claimed.')


def worker(args):
    global PARTIAL
    require(sys.platform=='linux' and sys.flags.optimize==0, 'Linux unoptimized-Python worker only')
    require(args.phase in ['qualify','validation','study'], 'Unsupported worker phase')
    output = args.output.resolve()
    prepared = read(output/'prepared.json')
    job = read(args.job)
    prepared_sha = digest(output/'prepared.json')
    require(args.prepared_sha==job['prepared_sha256']==prepared_sha, 'Job bound to another preparation')
    index = int(args.job.name.split('-')[0])
    require(job==dict(prepared['schedule'][args.phase][index],prepared_sha256=prepared_sha), 'Job differs from frozen schedule')
    require(args.label==job['label'], 'CLI label differs from scheduled job')
    require(sources()==prepared['observer_sources'], 'Observer source changed')
    require(environment()==prepared['child_environment'], 'Child environment differs')
    require(sorted(os.sched_getaffinity(0))==prepared['inherited_affinity'], 'Inherited CPU affinity changed')
    os.sched_setaffinity(0,{0})
    require(sorted(os.sched_getaffinity(0))==[0], 'CPU0 pin not applied')
    require(Path('/proc/sys/kernel/random/boot_id').read_text().strip()==prepared['boot_id'], 'New boot')
    image = prepared['images'][job['image']]
    artifact = output/'fixed-image.so'
    stat = artifact.stat()
    require(dict(device=stat.st_dev,inode=stat.st_ino)==prepared['scratch_identity'], 'File identity differs')
    raw = artifact.read_bytes()
    require(len(raw)==prepared['global_image_bytes'] and sha(raw)==image['padded_sha256'], 'Wrong installed image')
    require(sha(raw[:image['original_bytes']])==image['original_sha256'], 'Original prefix hash differs')
    require(raw[image['original_bytes']:]==bytes(image['padding_bytes']), 'Trailing padding differs')
    fixture = prepared['fixtures'][str(job['size'])]
    data = (Path(prepared['fixtures_directory'])/f"{job['size']}.bin").read_bytes()
    text = (Path(prepared['fixtures_directory'])/f"{job['size']}.txt").read_bytes()
    require(sha(data)==fixture['data_sha256'] and sha(text)==fixture['encoded_sha256'], 'Fixture changed')
    incoming,expected = (data,text) if job['operation']=='encode' else (text,data)
    capacity = max(len(incoming),len(expected)) if job['backend']=='empty' else len(expected)
    wanted = len(incoming) if job['backend']=='empty' else len(expected)
    library = Loaded(str(artifact))
    checked_name = 'size_'+job['operation']
    repeat_name = 'size_repeat_encode' if job['operation']=='encode' else 'size_repeat_decode_prevalidated'
    checked = getattr(library.real,checked_name)
    repeated = getattr(library.real,repeat_name)
    signature = (ctypes.c_void_p,ctypes.c_size_t,ctypes.c_void_p,ctypes.c_size_t)
    checked.argtypes,checked.restype = signature,ctypes.c_size_t
    repeated.argtypes,repeated.restype = (*signature,ctypes.c_size_t),ctypes.c_size_t
    pointer = ctypes.cast(repeated,ctypes.c_void_p).value
    symbol = library.elf.symbols[repeat_name]
    require(pointer==library.base+symbol['address'], 'Wrong timed export binding')
    section = library.elf.sections[symbol['index']]
    require(section[2]&4 and section[3]<=symbol['address'] and symbol['address']+symbol['size']<=section[3]+section[5], 'Timed export outside executable section')
    at = section[4]+symbol['address']-section[3]
    function = library.elf.data[at:at+symbol['size']]
    require(ctypes.string_at(pointer,symbol['size'])==function, 'Mapped timed function differs')
    mapped_before = library.mapped()
    require(all(row['identical'] for row in mapped_before.values()), 'Mapped code/readonly bytes differ')
    targets_before = library.relocations()
    encoder_pointer = ctypes.cast(library.real.size_encode,ctypes.c_void_p).value
    encoder_offset = library.elf.symbols['size_encode']['address']
    encoder_self_binding_before = verify_encoder_self_binding(library.base,encoder_offset,encoder_pointer,targets_before)
    mutable_before = mutable_state(library)
    source = (ctypes.c_ubyte*(len(incoming)+82))()
    storage = (ctypes.c_ubyte*(capacity+84))()
    ctypes.memset(source,0xa5,len(source))
    ctypes.memset(storage,0xa5,len(storage))
    ip_offset = offset(ctypes.addressof(source),1,1)
    op_offset = offset(ctypes.addressof(storage),3,3)
    ip,op = ctypes.addressof(source)+ip_offset,ctypes.addressof(storage)+op_offset
    ctypes.memmove(ip,incoming,len(incoming))
    require((ip%64,op%64)==(1,3), 'Wrong buffer residues')
    require(ip+len(incoming)<=op or op+capacity<=ip, 'Overlapping caller buffers')
    source_expected = b'\xa5'*ip_offset+incoming+b'\xa5'*(len(source)-ip_offset-len(incoming))

    def poison():
        ctypes.memset(op,0xa5,capacity)
        require(bytes(storage)[op_offset:op_offset+capacity]==b'\xa5'*capacity, 'Poison failed')

    def check(value):
        require(value==wanted, 'Wrong returned length')
        if job['backend']=='empty':
            require(bytes(storage)==b'\xa5'*len(storage), 'Scaffold changed output')
        else:
            require(bytes(storage)[op_offset:op_offset+wanted]==expected, 'Wrong output bytes')
        require(bytes(storage)[:op_offset]==b'\xa5'*op_offset, 'Leading canary changed')
        require(bytes(storage)[op_offset+capacity:]==b'\xa5'*(len(storage)-op_offset-capacity), 'Trailing canary changed')
        require(bytes(source)==source_expected, 'Input or its canaries changed')

    check(checked(ip,len(incoming),op,capacity))
    poison()
    require(repeated(ip,len(incoming),op,capacity,0)==0, 'repeat0 returned nonzero')
    require(bytes(storage)==b'\xa5'*len(storage) and bytes(source)==source_expected, 'repeat0 changed buffers/canaries')
    poison()
    one = repeated(ip,len(incoming),op,capacity,1)
    check(one)
    batches = []
    PARTIAL = dict(status='failed',job=job,batches=batches)
    if args.phase!='qualify':
        getcpu = ctypes.CDLL(None).sched_getcpu
        getcpu.argtypes,getcpu.restype = (),ctypes.c_int
        events_path = args.job.with_name(args.job.name.replace('-job.json','-batches.jsonl'))
        with events_path.open('x') as events:
            def event(value):
                events.write(json.dumps(value)+'\n')
                events.flush()
            iterations = 1
            while sum(row['accepted'] for row in batches)<SAMPLES:
                poison()
                arguments = (ip,len(incoming),op,capacity,iterations)
                before_cpu = getcpu()
                require(before_cpu==0, 'CPU changed before batch')
                event(dict(kind='begin',batch=len(batches),arguments=arguments,repeated_pointer=pointer))
                cpu_start,wall_start = time.process_time_ns(),time.perf_counter_ns()
                value = repeated(*arguments)
                cpu,wall = time.process_time_ns()-cpu_start,time.perf_counter_ns()-wall_start
                after_cpu = getcpu()
                accepted = cpu>=FLOOR
                row = dict(arguments=arguments,repeated_pointer=pointer,iterations=iterations,result=value,
                    cpu_ns=cpu,wall_ns=wall,accepted=accepted,cpu_before=before_cpu,cpu_after=after_cpu)
                batches.append(row)
                event(dict(kind='observed',batch=len(batches)-1,validation='pending',**row))
                require(after_cpu==0, 'CPU changed after batch')
                check(value)
                event(dict(kind='validated',batch=len(batches)-1))
                if not accepted:
                    require(iterations<CAP, 'Minimum CPU batch unreachable at declared cap')
                    iterations = min(CAP,iterations*max(2,min(16,(FLOOR*5)//(4*max(cpu,1)))))
    mapped_after = library.mapped()
    require(all(row['identical'] for row in mapped_after.values()), 'Mapped code/readonly bytes changed')
    require(ctypes.string_at(pointer,symbol['size'])==function, 'Timed function changed')
    targets_after = library.relocations()
    encoder_self_binding_after = verify_encoder_self_binding(library.base,encoder_offset,encoder_pointer,targets_after)
    require(digest(artifact)==image['padded_sha256'] and artifact.stat().st_ino==stat.st_ino, 'Installed file changed during child')
    require(sorted(os.sched_getaffinity(0))==[0], 'CPU mask changed')
    return dict(status='qualified' if args.phase=='qualify' else 'timed',job=job,
        scratch_identity=prepared['scratch_identity'],original_sha256=image['original_sha256'],
        padded_sha256=image['padded_sha256'],original_bytes=image['original_bytes'],padded_bytes=len(raw),
        repeated_export=repeat_name,repeated_pointer=pointer,repeated_module_offset=pointer-library.base,
        function_file_sha256=sha(function),function_memory_sha256=sha(ctypes.string_at(pointer,len(function))),
        checked_pointer=ctypes.cast(checked,ctypes.c_void_p).value,
        signature=[dict(type=item.__name__,bytes=ctypes.sizeof(item)) for item in repeated.argtypes],
        warmup_arguments=[ip,len(incoming),op,capacity,1],poisoned_repeat_one_result=one,
        poisoned_repeat_one_verified=True,input_sha256=sha(incoming),expected_sha256=sha(expected),
        input_pointer=ip,output_pointer=op,input_mod64=ip%64,output_mod64=op%64,
        input_mod4096=ip%4096,output_mod4096=op%4096,output_capacity_argument=capacity,
        mapped_before=mapped_before,mapped_after=mapped_after,targets_before=targets_before,
        targets_after=targets_after,mutable_before=mutable_before,mutable_after=mutable_state(library),
        encoder_self_binding_before=encoder_self_binding_before,encoder_self_binding_after=encoder_self_binding_after,
        diagnostic_scope='General GOT targets and writable CPU/cache objects are recorded for inspection, not verified against an expected state. Only parsed size_encode self-binding entries receive the explicit pointer/module-offset predicate.',
        environment=environment(),hash_seed_fingerprint=hash('faster-base58-fixed-inode-v2'),
        affinity=sorted(os.sched_getaffinity(0)),argv=sys.argv,pid=os.getpid(),ppid=os.getppid(),
        python=sys.version,python_executable=str(Path(sys.executable).resolve()),python_optimize=sys.flags.optimize,batches=batches,
        cpu_ns=statistics.median(row['cpu_ns']/row['iterations'] for row in batches if row['accepted']) if batches else None,
        wall_ns=statistics.median(row['wall_ns']/row['iterations'] for row in batches if row['accepted']) if batches else None,
        scope='New fixed-inode write/reload observer; warm conversion only. No historical-state or physical-mechanism claim.')


def discover(config):
    source = runpy.run_path(str(Path(config['source_root'])/'scripts/resource_report.py'))['source_identity']()
    require(source['sha256']==FINAL_SOURCE, 'Wrong final library source')
    expected_files = {item['path']:item['sha256'] for item in source['files']}
    images,contracts = {},{}
    for cpu in ['generic','native']:
        for policy in ['default','cache']:
            for variant in ['wide','slim']:
                cohort = '-'.join([cpu,policy,variant])
                folder = Path(config['linked_root'])/cohort
                metadata_path = folder/'metadata.json'
                metadata = read(metadata_path)
                require(metadata['target']=='x86_64-unknown-linux-gnu' and metadata['compact']==(variant=='slim')
                        and metadata['cache']==(policy=='cache') and metadata['count_allocations'] is False, 'Wrong consumer contract')
                require(metadata['rustflags']==('' if cpu=='generic' else '-C target-cpu=native'), 'Wrong CPU flags')
                for name,value in metadata['source_files_sha256'].items():
                    require(expected_files.get(name)==value, 'Consumer source mismatch: '+name)
                profile = tomllib.loads(metadata['profile'])['profile']['release']
                require(profile['opt-level']==3 and profile['lto']=='fat' and profile['codegen-units']==16, 'Wrong release profile')
                require('rustc 1.99.1' in metadata['rustc'] and 'LLVM version: 23.1.1' in metadata['rustc'], 'Wrong compiler')
                require(metadata['repeat_timing_contract']=='opaque-slice-lengths-v1'
                        and metadata['decode_timing_contract']=='prevalidated-utf8-repeat-v1', 'Wrong ABI observer')
                contracts[cohort] = dict(metadata_sha256=digest(metadata_path),sizes_sha256=digest(folder/'sizes.csv'))
                with (folder/'sizes.csv').open(newline='') as stream:
                    rows = list(csv.DictReader(stream))
                require(len(rows)==2 and {row['backend'] for row in rows}=={'auto','empty'}, 'Need exactly Auto/empty both-direction probes')
                for row in rows:
                    backend = row['backend']
                    features = (['cache'] if policy=='cache' else [])+(['compact'] if variant=='slim' else [])
                    require(metadata['selected_cargo_features'][backend]==features, 'Unexpected feature/internals instrumentation')
                    require(row['kind']=='cdylib' and row['direction']=='both' and row['count_allocations']=='False', 'Wrong artifact kind')
                    path = folder/(backend+'-both-cdylib.so')
                    require(digest(path)==row['artifact_sha256'], 'Original artifact hash differs')
                    images[cohort+'-'+backend] = dict(path=str(path.resolve()),original_sha256=row['artifact_sha256'],
                        original_bytes=path.stat().st_size,source_resource=FINAL_SOURCE,cohort=cohort,backend=backend)
    reference = Path(config['reference_path'])
    require(digest(reference)==REFERENCE_SHA, 'Wrong known identical-control artifact')
    images['reference'] = dict(path=str(reference.resolve()),original_sha256=REFERENCE_SHA,
        original_bytes=reference.stat().st_size,source_resource='07ec12777e6065bd644baa75af4fd2cd0fe38fc39674957dc1eb220dc220f3b1',backend='auto')
    require(len(images)==17, 'Wrong image inventory')
    global_length = max(image['original_bytes'] for image in images.values())
    for image in images.values():
        _,padding = padded_image(image['path'],image['original_sha256'],global_length)
        image.update(padding)
    return source,images,contracts,global_length


def guard(output=None, *, budget=False):
    require(sys.platform=='linux', 'Linux observer only')
    require(sys.flags.optimize==0, 'Require Python optimization level0')
    require(os.environ.get('PYTHONHASHSEED')=='0', 'Predeclared PYTHONHASHSEED=0 required')
    require(not os.environ.get('LD_PRELOAD') and not os.environ.get('LD_AUDIT'), 'Unexpected dynamic-loader instrumentation')
    if budget:
        require((DEADLINE-datetime.now(timezone.utc)).total_seconds()>=7200, 'Insufficient VM lifetime for this stage')
    if output is not None:
        prepared = read(output/'prepared.json')
        require(sources()==prepared['observer_sources'], 'Observer source changed')
        require(environment()==prepared['child_environment'], 'Parent runtime environment changed')
        require(sys.version==prepared['python'] and str(Path(sys.executable).resolve())==prepared['python_executable'], 'Python runtime changed')
        require(Path('/proc/sys/kernel/random/boot_id').read_text().strip()==prepared['boot_id'], 'Boot changed')
        require(shutil.disk_usage(output).free>=2*1024**3, '2GiB disk floor')
    return environment()


def prepare(args):
    guard(budget=True)
    config = read(args.inputs)
    output = args.output.resolve()
    require(not output.exists(), 'Refuse existing study destination')
    source,images,contracts,length = discover(config)
    fixtures_path = Path(config['fixture_manifest'])
    fixtures = read(fixtures_path)
    fixture_map = {str(row['decoded_bytes']):row for row in fixtures['fixtures']}
    require({int(key) for key in fixture_map}==set(SIZES), 'Wrong Natural fixture grid')
    for size,row in fixture_map.items():
        for suffix,key in [('bin','data_sha256'),('txt','encoded_sha256')]:
            require(digest(Path(config['fixtures_directory'])/(size+'.'+suffix))==row[key], 'Fixture hash mismatch')
    output.mkdir()
    scratch = Scratch(output/'fixed-image.so',length,create=True)
    try:
        prepared = dict(status='prepared; no conversions or timers',source_resource=source['sha256'],
            source_files=source['files'],inputs_path=str(args.inputs.resolve()),inputs_sha256=digest(args.inputs),config=config,images=images,contracts=contracts,
            global_image_bytes=length,scratch_identity=scratch.identity,observer_sources=sources(),
            fixtures_directory=str(Path(config['fixtures_directory']).resolve()),fixtures=fixture_map,
            fixture_manifest_sha256=digest(fixtures_path),boot_id=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),
            inherited_affinity=sorted(os.sched_getaffinity(0)),child_environment=environment(),
            python=sys.version,python_executable=str(Path(sys.executable).resolve()),python_optimize=sys.flags.optimize,
            schedule={phase:schedule(phase) for phase in ['qualify','validation','study']},
            seed=SEED,warm_samples=SAMPLES,fresh_pairs=REPEATS,minimum_cpu_batch_ns=FLOOR,iteration_cap=CAP,
            lifetime_deadline_utc=DEADLINE.isoformat(),scope='One scratchpath/inode/global length across all17images and fourCPU/cachecohorts; immutable originals retained.')
        require(prepared['inherited_affinity']==[0,1,2,3], 'Unexpected VM affinity')
        create(output/'prepared.json',prepared)
    finally:
        scratch.close()


def run_phase(args):
    output = args.output.resolve()
    guard(output,budget=True)
    prepared = read(output/'prepared.json')
    require(args.prepared_sha==digest(output/'prepared.json'), 'Preparation differs from reviewed hash')
    require(digest(prepared['inputs_path'])==prepared['inputs_sha256'], 'Inputs configuration changed')
    current_source = runpy.run_path(str(Path(prepared['config']['source_root'])/'scripts/resource_report.py'))['source_identity']()
    require(current_source==dict(sha256=FINAL_SOURCE,files=prepared['source_files']), 'Source tree changed')
    for cohort,contract in prepared['contracts'].items():
        folder = Path(prepared['config']['linked_root'])/cohort
        require(digest(folder/'metadata.json')==contract['metadata_sha256'] and digest(folder/'sizes.csv')==contract['sizes_sha256'], 'Build contract changed')
    snapshot = subprocess.check_output(['ps','-e','-o','pid,ppid,pgid,sid,stat,comm'],text=True)
    forbidden = ['cargo','rustc','clang','cc1','ld.lld','cbmc','z3','timing','counted','miri','encode','decode']
    require(not any(line.split()[-1] in forbidden for line in snapshot.splitlines()[1:]), 'Other compiler/test/fuzzer/worker active')
    if args.phase!='qualify':
        require(read(output/'qualify/complete.json')['status']=='complete', 'Untimed qualification required')
    if args.phase=='study':
        require(read(output/'validation/complete.json')['status']=='complete', 'Separate validation/root review required')
    folder = output/args.phase
    folder.mkdir()
    planned = prepared['schedule'][args.phase]
    require(planned==schedule(args.phase), 'Schedule differs')
    create(folder/'protocol.json',dict(phase=args.phase,jobs=planned,prepared_sha256=digest(output/'prepared.json'),observer_sources=sources(),process_snapshot=snapshot))
    scratch = Scratch(output/'fixed-image.so',prepared['global_image_bytes'],create=False,expected_identity=prepared['scratch_identity'])
    from process_supervisor import install_signal_handlers,owned_process
    install_signal_handlers()
    started = time.monotonic()
    completed = 0
    current = None
    try:
        with (folder/'raw.jsonl').open('x') as raw:
            for index,item in enumerate(planned):
                current = dict(index=index,job=item)
                guard(output)
                image = prepared['images'][item['image']]
                installed = scratch.install(image['path'],image['original_sha256'])
                require(installed['padded_sha256']==image['padded_sha256'], 'Installed image differs from plan')
                job_path,log = folder/f'{index:05d}-job.json',folder/f'{index:05d}-worker.json'
                create(job_path,dict(**item,prepared_sha256=digest(output/'prepared.json')))
                command = [sys.executable,str(Path(__file__).resolve()),'--worker','--output',str(output),
                           '--phase',args.phase,'--job',str(job_path),'--label',item['label'],'--prepared-sha',args.prepared_sha]
                with scratch.child_load_window():
                    with log.open('x') as stream:
                        with owned_process(command,cwd=output,env=dict(os.environ,PYTHONDONTWRITEBYTECODE='1'),
                            stdin=subprocess.DEVNULL,stdout=stream,stderr=subprocess.STDOUT) as child:
                            begin = time.monotonic()
                            while child.poll() is None:
                                require((DEADLINE-datetime.now(timezone.utc)).total_seconds()>300,
                                        'VM lifetime300s reserve reached; incomplete child cleanup/reap required')
                                if time.monotonic()-begin>1800:
                                    raise RuntimeError('Administrative process timeout; partial log retained')
                                time.sleep(.05)
                            require(child.returncode==0, 'Worker failed: '+str(log))
                require(sha(scratch.read())==image['padded_sha256'], 'Scratch changed after child reap')
                result = read(log)
                require(result['scratch_identity']==prepared['scratch_identity'] and result['poisoned_repeat_one_verified'], 'Child identity/poison guard missing')
                require(result['job']==dict(item,prepared_sha256=args.prepared_sha), 'Returned job differs from scheduled invocation')
                require(result['original_sha256']==image['original_sha256'] and result['padded_sha256']==image['padded_sha256'], 'Returned image differs from scheduled variant')
                raw.write(json.dumps(dict(**item,worker=result))+'\n')
                raw.flush()
                completed += 1
                print('FIXED-INODE',args.phase,index+1,'/',len(planned),item['study'],item['cpu'],item['policy'],item['operation'],item['size'],flush=True)
        guard(output)
        require(digest(prepared['inputs_path'])==prepared['inputs_sha256'], 'Inputs configuration changed')
        require(runpy.run_path(str(Path(prepared['config']['source_root'])/'scripts/resource_report.py'))['source_identity']()==current_source, 'Source tree changed')
        require(digest(prepared['config']['fixture_manifest'])==prepared['fixture_manifest_sha256'], 'Fixture manifest changed')
        for image in prepared['images'].values():
            require(digest(image['path'])==image['original_sha256'], 'Immutable original changed')
        create(folder/'complete.json',dict(status='complete',phase=args.phase,processes=len(planned),
            elapsed_seconds=time.monotonic()-started,raw_sha256=digest(folder/'raw.jsonl'),
            prepared_sha256=digest(output/'prepared.json'),observer_sources=sources(),scratch_identity=prepared['scratch_identity']))
    except BaseException as error:
        create(folder/'incomplete.json',dict(status='incomplete',phase=args.phase,current=current,
            completed_processes=completed,error=repr(error),lifetime_reserve_seconds=300,
            partial_scope='Every flushed begin/observed/validated batch event remains separate; pending events are not qualified observations. No complete marker or selected retry.',
            cleanup_scope='Parent exception exits owned_process before this receipt. The helper signals the owned private group and waits/reaps its direct child; independent descendant reaping is not proved. Declared observer workers have no asynchronous descendants. Audit actual process state before another owner starts.'))
        raise
    finally:
        scratch.close()


def launch_phase(args):
    output = args.output.resolve()
    guard(output,budget=True)
    require(args.phase in ['qualify','validation','study'], 'Only explicit conversion stages can launch')
    require(args.prepared_sha==digest(output/'prepared.json'), 'Preparation differs from reviewed hash')
    receipt = output/(args.phase+'-launch.json')
    log = output/(args.phase+'-run.log')
    require(not receipt.exists() and not log.exists(), 'Refuse an already launched stage')
    command = [sys.executable,str(Path(__file__).resolve()),'--phase',args.phase,'--output',str(output),'--prepared-sha',args.prepared_sha]
    with log.open('xb') as stream:
        # The detached bounded controller, not this returning launcher, owns
        # measurement children and their cleanup/reap contexts.
        child = subprocess.Popen(command,cwd=output,env=dict(os.environ,PYTHONDONTWRITEBYTECODE='1'),
            start_new_session=True,stdin=subprocess.DEVNULL,stdout=stream,stderr=subprocess.STDOUT,close_fds=True)
    stat = Path(f'/proc/{child.pid}/stat').read_text().split(') ',1)[1].split()
    require(int(stat[2])==child.pid==int(stat[3]), 'Controller not in its own session/group')
    create(receipt,dict(status='durable stage controller launched',pid=child.pid,start_ticks=int(stat[19]),
        pgid=int(stat[2]),sid=int(stat[3]),command=command,stdin='DEVNULL',stdout=log.name,stderr='STDOUT',
        observer_sources=sources(),prepared_sha256=args.prepared_sha))
    print('LAUNCHED',args.phase,child.pid,flush=True)


if __name__=='__main__':
    if sys.flags.optimize:
        raise SystemExit('Require Python optimization level0')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--phase',choices=['prepare','qualify','validation','study'],required=True)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--inputs',type=Path)
    parser.add_argument('--worker',action='store_true')
    parser.add_argument('--job',type=Path)
    parser.add_argument('--label',choices=['alpha','bravo'])
    parser.add_argument('--prepared-sha')
    parser.add_argument('--launch',action='store_true')
    options = parser.parse_args()
    if options.worker:
        try:
            print(json.dumps(worker(options)))
        except BaseException:
            print(json.dumps(PARTIAL),flush=True)
            raise
    elif options.phase=='prepare':
        require(not options.launch, 'Preparation never launches conversions')
        prepare(options)
    elif options.launch:
        launch_phase(options)
    else:
        run_phase(options)
