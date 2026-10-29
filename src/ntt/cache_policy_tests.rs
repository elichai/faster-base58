use super::*;
#[cfg(feature = "cache")]
use core::cell::Cell;

#[cfg(feature = "cache")]
const CACHE_LIMIT: u32 = CACHE_MAX_LOG;

#[test]
fn feature_policy_and_owned_override_select_table_source() {
    for (name, mut ntt, owned) in [
        (
            "feature",
            Ntt::new(Kernels::portable()),
            !cfg!(feature = "cache"),
        ),
        (
            "cached control",
            Ntt::with_cache(true, Kernels::portable()),
            !cfg!(feature = "cache"),
        ),
        (
            "owned control",
            Ntt::with_cache(false, Kernels::portable()),
            true,
        ),
    ] {
        for log in [5, 12, 8] {
            ntt.prepare(1usize << (log - 1));
            assert!(
                ntt.owned.is_none(),
                "{name}: prepare must discard owned tables"
            );
            #[cfg(feature = "cache")]
            assert!(
                ntt.shared.is_none(),
                "{name}: prepare must discard shared selection"
            );
            ntt.ensure_tables();
            let table = ntt.initialized_tables();
            assert_eq!(ntt.owned.is_some(), owned, "{name}, log {log}");
            if let Some(local) = ntt.owned.as_ref() {
                assert!(
                    core::ptr::eq(table, local),
                    "{name}: table must come from this multiplier"
                );
                assert_eq!(table.log, log);
            }
            #[cfg(feature = "cache")]
            {
                assert_eq!(ntt.shared.is_some(), !owned, "{name}, log {log}");
                if let Some(shared) = ntt.shared {
                    assert!(
                        core::ptr::eq(table, shared),
                        "{name}: table must come from the cache"
                    );
                    assert!(table.log >= log);
                }
            }
        }
    }
}

#[test]
fn unprepared_power_fallback_keeps_tables_lazy() {
    let (a, b) = ([u64::MAX, 1], [17, 23]);
    let mut want = [0; 4];
    let mut scratch = vec![0; scratch_len(2)];
    mul::mul(&mut want, &a, &b, &mut scratch);
    for cached in [false, true] {
        let mut ntt = Ntt::with_cache(cached, Kernels::portable());
        ntt.prepare(4);
        for index in [7, 0] {
            if index == 0 {
                // This power is below the preparation threshold, so it stays unprepared.
                ntt.prepare_power(0, &b, 4);
            }
            let mut out = [0; 4];
            ntt.mul_power(&mut out, &a, index, &b, &mut scratch);
            assert_eq!(out, want);
            assert!(ntt.owned.is_none(), "cached={cached}, index={index}");
            #[cfg(feature = "cache")]
            assert!(ntt.shared.is_none(), "cached={cached}, index={index}");
        }
    }
}

#[cfg(not(feature = "cache"))]
#[test]
fn requested_tables_are_owned_without_cache() {
    for log in [5, 12] {
        let mut first = Ntt::new(Kernels::portable());
        let mut second = Ntt::new(Kernels::portable());
        first.prepare(1usize << (log - 1));
        second.prepare(1usize << (log - 1));
        first.ensure_tables();
        second.ensure_tables();
        let (a, b) = (first.initialized_tables(), second.initialized_tables());
        assert!(first.owned.is_some() && second.owned.is_some());
        assert_eq!(a.log, log);
        assert_eq!(b.log, log);
        assert_eq!(a.tw[0].fw, b.tw[0].fw);
        assert_ne!(a.tw[0].fw.as_ptr(), b.tw[0].fw.as_ptr());
    }
    #[cfg(feature = "__internals")]
    {
        let (header, rows) = internals::diagnostics::ntt_cache_memory();
        assert_eq!(header, 0);
        assert!(rows.iter().all(|r| !r.1 && r.2 == 0 && r.3 == 0));
    }
}

#[cfg(feature = "cache")]
#[test]
fn large_tables_are_owned_without_splitting_transforms() {
    thread_local!(static LARGEST: Cell<usize> = const { Cell::new(0) });
    let kernels = Kernels {
        forward: |a, p, t| {
            LARGEST.with(|n| n.set(n.get().max(a.len())));
            portable::forward(a, p, t);
        },
        ..Kernels::portable()
    };
    let log = CACHE_LIMIT + 1;
    let n = 1 << (log - 2);
    let mut ntt = Ntt::new(kernels);
    ntt.prepare(2 * n);
    ntt.ensure_tables();
    assert_eq!(ntt.initialized_tables().log, log);
    assert!(ntt.owned.is_some(), "large twiddles must leave the cache");
    assert!(ntt.shared.is_none());

    let a = vec![u64::MAX; n];
    let b = a.clone();
    ntt.prepare_power(0, &b, 2 * n);
    assert_eq!(ntt.powers[0].as_ref().unwrap().log, log);
    let mut out = vec![0; 2 * n];
    ntt.mul_power(&mut out, &a, 0, &b, &mut vec![0; scratch_len(n)]);
    // (beta^n - 1)^2 = beta^(2n) - 2 beta^n + 1.
    assert_eq!(out[0], 1);
    assert!(out[1..n].iter().all(|&x| x == 0));
    assert_eq!(out[n], u64::MAX - 1);
    assert!(out[n + 1..].iter().all(|&x| x == u64::MAX));
    assert_eq!(LARGEST.with(Cell::get), 1 << log);

    ntt.prepare(64);
    assert!(ntt.owned.is_none() && ntt.shared.is_none());
    assert!(
        ntt.powers.is_empty(),
        "prepare must discard old forward transforms"
    );
    ntt.ensure_tables();
    assert!((7..=CACHE_LIMIT).contains(&ntt.initialized_tables().log));
    assert!(ntt.owned.is_none() && ntt.shared.is_some());
}

#[cfg(feature = "cache")]
#[test]
fn owned_mode_stays_owned_after_cache_initialization() {
    Tables::cached(12);
    let mut ntt = Ntt::new_owned(Kernels::portable());
    for log in [12, 8, 13] {
        ntt.prepare(1 << (log - 1));
        assert!(ntt.owned.is_none() && ntt.shared.is_none());
        ntt.ensure_tables();
        assert_eq!(ntt.initialized_tables().log, log);
        assert!(ntt.owned.is_some() && ntt.shared.is_none());
    }
}

#[cfg(all(feature = "cache", feature = "__internals"))]
#[test]
fn retention_is_bounded_across_histories() {
    const CHILD: &str = "BASE58_NTT_CACHE_HISTORY";
    const NAME: &str = "ntt::cache_policy_tests::retention_is_bounded_across_histories";
    let Ok(history) = std::env::var(CHILD) else {
        for history in ["ascending", "descending", "mixed", "concurrent"] {
            let child = std::process::Command::new(std::env::current_exe().unwrap())
                .args([NAME, "--exact", "--nocapture"])
                .env(CHILD, history)
                .output()
                .unwrap();
            assert!(
                child.status.success(),
                "{history}: {}{}",
                String::from_utf8_lossy(&child.stdout),
                String::from_utf8_lossy(&child.stderr)
            );
        }
        return;
    };
    assert!(
        internals::diagnostics::ntt_cache_memory()
            .1
            .iter()
            .all(|r| !r.1)
    );
    let bound = 24 * ((1usize << (CACHE_LIMIT + 1)) - 32);
    let touch = |log: u32| {
        let mut ntt = Ntt::new(Kernels::portable());
        ntt.prepare(1usize << (log - 1));
        ntt.ensure_tables();
        let t = ntt.initialized_tables();
        assert!(t.log >= log);
        assert_eq!(ntt.owned.is_some(), log > CACHE_LIMIT);
        assert_eq!(ntt.shared.is_some(), log <= CACHE_LIMIT);
        for tw in &t.tw {
            for v in [&tw.fw, &tw.iw] {
                assert_eq!(v.len(), 1 << t.log);
                assert_eq!(v.capacity(), v.len());
            }
        }
        let rows = internals::diagnostics::ntt_cache_memory().1;
        assert!(rows.iter().filter(|r| r.0 > CACHE_LIMIT).all(|r| !r.1));
        assert!(rows.iter().map(|r| r.3).sum::<usize>() <= bound);
    };
    match history.as_str() {
        "ascending" => (5..=CACHE_LIMIT + 1).for_each(touch),
        "descending" => (5..=CACHE_LIMIT + 1).rev().for_each(touch),
        "mixed" => [
            5,
            CACHE_LIMIT + 1,
            CACHE_LIMIT,
            6,
            CACHE_LIMIT + 1,
            CACHE_LIMIT - 1,
            5,
        ]
        .into_iter()
        .for_each(touch),
        "concurrent" => {
            let start = std::sync::Barrier::new(8);
            std::thread::scope(|scope| {
                let threads: Vec<_> = [
                    5,
                    8,
                    CACHE_LIMIT,
                    CACHE_LIMIT - 1,
                    CACHE_LIMIT,
                    12,
                    CACHE_LIMIT + 1,
                    8,
                ]
                .into_iter()
                .map(|log| {
                    let (start, touch) = (&start, &touch);
                    scope.spawn(move || {
                        start.wait();
                        touch(log);
                    })
                })
                .collect();
                for t in threads {
                    t.join().unwrap();
                }
            });
        }
        _ => panic!("unknown cache history {history}"),
    }
    let rows = internals::diagnostics::ntt_cache_memory().1;
    let retained = rows.iter().map(|r| r.3).sum::<usize>();
    match history.as_str() {
        "ascending" => assert_eq!(retained, bound),
        "descending" => assert_eq!(retained, 24 << CACHE_LIMIT),
        "mixed" => assert_eq!(retained, 24 * ((1 << CACHE_LIMIT) + 32)),
        _ => assert!(retained <= bound),
    }
    // Once the maximum cacheable table exists, all smaller requests reuse it.
    (5..=CACHE_LIMIT).for_each(touch);
    assert_eq!(rows, internals::diagnostics::ntt_cache_memory().1);
}
