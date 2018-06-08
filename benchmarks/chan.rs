#[macro_use]
extern crate chan;
extern crate crossbeam;

const MESSAGES: usize = 5_000_000;
const THREADS: usize = 4;

fn new<T>(cap: Option<usize>) -> (chan::Sender<T>, chan::Receiver<T>) {
    match cap {
        None => chan::async(),
        Some(cap) => chan::sync(cap)
    }
}

fn seq(cap: Option<usize>) {
    let (tx, rx) = new::<i32>(cap);

    for i in 0..MESSAGES {
        tx.send(i as i32);
    }

    for _ in 0..MESSAGES {
        rx.recv().unwrap();
    }
}

fn spsc(cap: Option<usize>) {
    let (tx, rx) = new::<i32>(cap);

    crossbeam::scope(|s| {
        s.spawn(|| {
            for i in 0..MESSAGES {
                tx.send(i as i32);
            }
        });

        for _ in 0..MESSAGES {
            rx.recv().unwrap();
        }
    });
}

fn mpsc(cap: Option<usize>) {
    let (tx, rx) = new::<i32>(cap);

    crossbeam::scope(|s| {
        for _ in 0..THREADS {
            s.spawn(|| {
                for i in 0..MESSAGES / THREADS {
                    tx.send(i as i32);
                }
            });
        }

        for _ in 0..MESSAGES {
            rx.recv().unwrap();
        }
    });
}

fn mpmc(cap: Option<usize>) {
    let (tx, rx) = new::<i32>(cap);

    crossbeam::scope(|s| {
        for _ in 0..THREADS {
            s.spawn(|| {
                for i in 0..MESSAGES / THREADS {
                    tx.send(i as i32);
                }
            });
        }

        for _ in 0..THREADS {
            s.spawn(|| {
                for _ in 0..MESSAGES / THREADS {
                    rx.recv().unwrap();
                }
            });
        }
    });
}

fn select_rx(cap: Option<usize>) {
    assert_eq!(THREADS, 4);
    let chans = (0..THREADS).map(|_| new::<i32>(cap)).collect::<Vec<_>>();

    crossbeam::scope(|s| {
        for (tx, _) in &chans {
            let tx = tx.clone();
            s.spawn(move || {
                for i in 0..MESSAGES / THREADS {
                    tx.send(i as i32);
                }
            });
        }

        let rx0 = &chans[0].1;
        let rx1 = &chans[1].1;
        let rx2 = &chans[2].1;
        let rx3 = &chans[3].1;

        for _ in 0..MESSAGES {
            chan_select! {
                rx0.recv() -> m => assert!(m.is_some()),
                rx1.recv() -> m => assert!(m.is_some()),
                rx2.recv() -> m => assert!(m.is_some()),
                rx3.recv() -> m => assert!(m.is_some()),
            }
        }
    });
}

fn select_both(cap: Option<usize>) {
    assert_eq!(THREADS, 4);
    let chans = (0..THREADS).map(|_| new::<i32>(cap)).collect::<Vec<_>>();

    crossbeam::scope(|s| {
        for _ in 0..THREADS {
            let chans = chans.clone();
            s.spawn(move || {
                let tx0 = &chans[0].0;
                let tx1 = &chans[1].0;
                let tx2 = &chans[2].0;
                let tx3 = &chans[3].0;
                for i in 0..MESSAGES / THREADS {
                    chan_select! {
                        tx0.send(i as i32) => {},
                        tx1.send(i as i32) => {},
                        tx2.send(i as i32) => {},
                        tx3.send(i as i32) => {},
                    }
                }
            });
        }

        for _ in 0..THREADS {
            let chans = chans.clone();
            s.spawn(move || {
                let rx0 = &chans[0].1;
                let rx1 = &chans[1].1;
                let rx2 = &chans[2].1;
                let rx3 = &chans[3].1;
                for _ in 0..MESSAGES / THREADS {
                    chan_select! {
                        rx0.recv() -> m => assert!(m.is_some()),
                        rx1.recv() -> m => assert!(m.is_some()),
                        rx2.recv() -> m => assert!(m.is_some()),
                        rx3.recv() -> m => assert!(m.is_some()),
                    }
                }
            });
        }
    });
}

fn main() {
    macro_rules! run {
        ($name:expr, $f:expr) => {
            let now = ::std::time::Instant::now();
            $f;
            let elapsed = now.elapsed();
            println!(
                "{:25} {:15} {:7.3} sec",
                $name,
                "Rust chan",
                elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 / 1e9
            );
        }
    }

    run!("bounded0_mpmc", mpmc(Some(0)));
    run!("bounded0_mpsc", mpsc(Some(0)));
    run!("bounded0_select_both", select_both(Some(0)));
    run!("bounded0_select_rx", select_rx(Some(0)));
    run!("bounded0_spsc", spsc(Some(0)));

    run!("bounded1_mpmc", mpmc(Some(1)));
    run!("bounded1_mpsc", mpsc(Some(1)));
    run!("bounded1_select_both", select_both(Some(1)));
    run!("bounded1_select_rx", select_rx(Some(1)));
    run!("bounded1_spsc", spsc(Some(1)));

    run!("bounded_mpmc", mpmc(Some(MESSAGES)));
    run!("bounded_mpsc", mpsc(Some(MESSAGES)));
    run!("bounded_select_both", select_both(Some(MESSAGES)));
    run!("bounded_select_rx", select_rx(Some(MESSAGES)));
    run!("bounded_seq", seq(Some(MESSAGES)));
    run!("bounded_spsc", spsc(Some(MESSAGES)));

    run!("unbounded_mpmc", mpmc(None));
    run!("unbounded_mpsc", mpsc(None));
    run!("unbounded_select_both", select_both(None));
    run!("unbounded_select_rx", select_rx(None));
    run!("unbounded_seq", seq(None));
    run!("unbounded_spsc", spsc(None));
}
