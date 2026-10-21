//! A tiny fork-join pool for work inside one token: the main thread posts a job, every worker
//! runs it with its own index, the main thread runs index 0 and waits. Workers spin (a token is
//! ~50 µs, a park/unpark round trip is more than that) and fall back to parking when idle.
//! On targets without threads (wasm) `Pool::new(n)` gives a pool of one and runs jobs inline.
use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use std::sync::Arc;

type Job<'a> = &'a (dyn Fn(usize) + Sync);

struct Shared {
    gen: AtomicUsize,          // bumped by the main thread for every job
    done: AtomicUsize,         // workers finished with the current job
    job: std::cell::UnsafeCell<Option<Job<'static>>>,
    quit: AtomicBool,
    parked: AtomicUsize,
}
unsafe impl Sync for Shared {}

pub struct Pool { n: usize, sh: Arc<Shared>, workers: Vec<std::thread::JoinHandle<()>> }

impl Pool {
    pub fn new(n: usize) -> Pool {
        let n = n.max(1);
        let sh = Arc::new(Shared { gen: AtomicUsize::new(0), done: AtomicUsize::new(0), job: std::cell::UnsafeCell::new(None), quit: AtomicBool::new(false), parked: AtomicUsize::new(0) });
        let mut workers = Vec::new();
        #[cfg(not(target_arch = "wasm32"))]
        for tid in 1..n {
            let sh = sh.clone();
            workers.push(std::thread::Builder::new().name(format!("slm-{tid}")).spawn(move || {
                qos_interactive();
                let mut seen = 0usize;
                loop {
                    let mut spins = 0u32;
                    loop {
                        let g = sh.gen.load(Ordering::Acquire);
                        if g != seen { seen = g; break; }
                        if sh.quit.load(Ordering::Relaxed) { return; }
                        spins += 1;
                        if spins < 200_000 { std::hint::spin_loop(); } else { sh.parked.fetch_add(1, Ordering::SeqCst); std::thread::park_timeout(std::time::Duration::from_micros(200)); sh.parked.fetch_sub(1, Ordering::SeqCst); }
                    }
                    let job = unsafe { (*sh.job.get()).unwrap() };
                    job(tid);
                    sh.done.fetch_add(1, Ordering::Release);
                }
            }).expect("spawn"));
        }
        let n = workers.len() + 1;
        Pool { n, sh, workers }
    }
    pub fn threads(&self) -> usize { self.n }
    /// Run `f(tid)` for tid in 0..threads(), returning when all are done.
    #[inline] pub fn run(&self, f: &(dyn Fn(usize) + Sync)) {
        if self.n == 1 { f(0); return; }
        let job: Job<'static> = unsafe { std::mem::transmute::<Job<'_>, Job<'static>>(f) };
        unsafe { *self.sh.job.get() = Some(job); }
        self.sh.done.store(0, Ordering::Relaxed);
        self.sh.gen.fetch_add(1, Ordering::AcqRel);
        if self.sh.parked.load(Ordering::SeqCst) > 0 { for w in &self.workers { w.thread().unpark(); } }
        f(0);
        while self.sh.done.load(Ordering::Acquire) < self.n - 1 { std::hint::spin_loop(); }
        unsafe { *self.sh.job.get() = None; }
    }
}
impl Drop for Pool {
    fn drop(&mut self) {
        self.sh.quit.store(true, Ordering::Relaxed);
        for w in &self.workers { w.thread().unpark(); }
        for w in self.workers.drain(..) { let _ = w.join(); }
    }
}

/// Ask macOS to schedule this thread on performance cores.
#[cfg(target_os = "macos")]
fn qos_interactive() {
    extern "C" { fn pthread_set_qos_class_self_np(cls: u32, rel: i32) -> i32; }
    unsafe { pthread_set_qos_class_self_np(0x21 /* QOS_CLASS_USER_INTERACTIVE */, 0); }
}
#[cfg(not(target_os = "macos"))]
fn qos_interactive() {}

/// Rows [0, n) split into `t` ranges aligned to `align`; returns the range of part `i`.
#[inline] pub fn split(n: usize, t: usize, i: usize, align: usize) -> (usize, usize) {
    let per = (n + t - 1) / t;
    let per = (per + align - 1) / align * align;
    let a = (i * per).min(n); let b = ((i + 1) * per).min(n);
    (a, b)
}

/// A pointer workers can write through on disjoint ranges. Caller guarantees the ranges don't overlap.
#[derive(Clone, Copy)] pub struct Shelf<T>(pub *mut T, pub usize);
unsafe impl<T> Send for Shelf<T> {}
unsafe impl<T> Sync for Shelf<T> {}
impl<T> Shelf<T> {
    #[inline] pub fn new(s: &mut [T]) -> Shelf<T> { Shelf(s.as_mut_ptr(), s.len()) }
    /// `&mut` to [a, b): the caller promises no other thread touches this range during the job.
    #[inline] #[allow(clippy::mut_from_ref)] pub unsafe fn range(&self, a: usize, b: usize) -> &mut [T] { debug_assert!(a <= b && b <= self.1); std::slice::from_raw_parts_mut(self.0.add(a), b - a) }
    #[inline] pub fn all(&self) -> &[T] { unsafe { std::slice::from_raw_parts(self.0, self.1) } }
}
