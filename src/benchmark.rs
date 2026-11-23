use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// Motor de processamento real de CPU que converte ciclos (SHA-256 + Tensores de Lorentz FMA)
/// em energia de feixe e entropia quântica verificável.
pub struct CpuComputePool {
    max_threads: usize,
    active_threads: Arc<AtomicUsize>,
    intensity_pct: Arc<AtomicU8>,
    running: Arc<AtomicBool>,
    total_hashes: Arc<AtomicU64>,
    total_flops: Arc<AtomicU64>,
    entropy_pool: Arc<AtomicU64>,
    last_measure_instant: Instant,
    last_hashes: u64,
    last_flops: u64,
}

impl CpuComputePool {
    pub fn new(initial_threads: usize, initial_intensity: u8) -> Self {
        let max_threads = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .max(1);
        let clamped_threads = initial_threads.clamp(1, max_threads);
        let clamped_intensity = initial_intensity.clamp(5, 100);

        let active_threads = Arc::new(AtomicUsize::new(clamped_threads));
        let intensity_pct = Arc::new(AtomicU8::new(clamped_intensity));
        let running = Arc::new(AtomicBool::new(true));
        let total_hashes = Arc::new(AtomicU64::new(0));
        let total_flops = Arc::new(AtomicU64::new(0));
        let entropy_pool = Arc::new(AtomicU64::new(0xC011_1DE2_2026_CAFE));

        for worker_id in 0..max_threads {
            let active_threads = Arc::clone(&active_threads);
            let intensity_pct = Arc::clone(&intensity_pct);
            let running = Arc::clone(&running);
            let total_hashes = Arc::clone(&total_hashes);
            let total_flops = Arc::clone(&total_flops);
            let entropy_pool = Arc::clone(&entropy_pool);

            thread::Builder::new()
                .name(format!("collider-rf-cavity-{}", worker_id))
                .spawn(move || {
                    worker_loop(
                        worker_id,
                        active_threads,
                        intensity_pct,
                        running,
                        total_hashes,
                        total_flops,
                        entropy_pool,
                    );
                })
                .expect("Falha ao iniciar thread de cavidade RF");
        }

        Self {
            max_threads,
            active_threads,
            intensity_pct,
            running,
            total_hashes,
            total_flops,
            entropy_pool,
            last_measure_instant: Instant::now(),
            last_hashes: 0,
            last_flops: 0,
        }
    }

    pub fn max_threads(&self) -> usize {
        self.max_threads
    }

    pub fn active_threads(&self) -> usize {
        self.active_threads.load(Ordering::Relaxed)
    }

    pub fn set_active_threads(&self, count: usize) {
        self.active_threads
            .store(count.clamp(1, self.max_threads), Ordering::Relaxed);
    }

    pub fn intensity_pct(&self) -> u8 {
        self.intensity_pct.load(Ordering::Relaxed)
    }

    pub fn set_intensity_pct(&self, pct: u8) {
        self.intensity_pct
            .store(pct.clamp(5, 100), Ordering::Relaxed);
    }

    pub fn entropy_seed(&self) -> u64 {
        self.entropy_pool.load(Ordering::Relaxed)
    }

    pub fn total_hashes(&self) -> u64 {
        self.total_hashes.load(Ordering::Relaxed)
    }

    /// Coleta a taxa real de Hashes/s e GFLOPS desde a última medição.
    pub fn sample_rates(&mut self) -> (f64, f64, u64) {
        let now = Instant::now();
        let dt = now
            .duration_since(self.last_measure_instant)
            .as_secs_f64()
            .max(1e-4);
        let cur_hashes = self.total_hashes.load(Ordering::Relaxed);
        let cur_flops = self.total_flops.load(Ordering::Relaxed);

        let d_hashes = cur_hashes.saturating_sub(self.last_hashes);
        let d_flops = cur_flops.saturating_sub(self.last_flops);

        self.last_measure_instant = now;
        self.last_hashes = cur_hashes;
        self.last_flops = cur_flops;

        let hashes_per_sec = d_hashes as f64 / dt;
        let gflops = (d_flops as f64 / dt) / 1e9;
        (hashes_per_sec, gflops, cur_hashes)
    }
}

impl Drop for CpuComputePool {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

fn worker_loop(
    worker_id: usize,
    active_threads: Arc<AtomicUsize>,
    intensity_pct: Arc<AtomicU8>,
    running: Arc<AtomicBool>,
    total_hashes: Arc<AtomicU64>,
    total_flops: Arc<AtomicU64>,
    entropy_pool: Arc<AtomicU64>,
) {
    let mut nonce: u64 = (worker_id as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    // Vetor de estado relativístico 4D (E/c, px, py, pz) simulado em ponto flutuante
    let mut p = [
        1.0 + worker_id as f64 * 0.13,
        0.42,
        -0.31,
        0.89,
    ];

    let cycle_window = Duration::from_millis(25);

    while running.load(Ordering::Relaxed) {
        if worker_id >= active_threads.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(60));
            continue;
        }

        let pct = intensity_pct.load(Ordering::Relaxed).clamp(5, 100) as u64;
        let work_budget = Duration::from_micros((cycle_window.as_micros() as u64 * pct) / 100);
        let sleep_budget = cycle_window.saturating_sub(work_budget);

        let burst_start = Instant::now();
        let mut local_hashes: u64 = 0;
        let mut local_flops: u64 = 0;
        let mut local_entropy: u64 = 0;

        while burst_start.elapsed() < work_budget {
            // Bloco de 64 iterações combinando SHA-256 real + transformações de Lorentz 4x4 FMA
            for _ in 0..64 {
                nonce = nonce.wrapping_add(1);
                let mut hasher = Sha256::new();
                hasher.update(nonce.to_le_bytes());
                hasher.update(p[0].to_bits().to_le_bytes());
                let digest = hasher.finalize();

                let lane = u64::from_le_bytes([
                    digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6],
                    digest[7],
                ]);
                local_entropy ^= lane;
                local_hashes += 1;

                // 32 rodadas de álgebra de feixe (16 FLOPs por rodada = 512 FLOPs por hash)
                let angle = ((lane & 0xFFFF) as f64) * 1e-5;
                let s = angle.sin();
                let c = angle.cos();
                for _ in 0..32 {
                    let e = p[0];
                    let px = p[1];
                    let py = p[2];
                    let pz = p[3];
                    p[0] = e * 1.000001 + pz * 0.000001;
                    p[1] = px * c - py * s + 1e-7;
                    p[2] = px * s + py * c - 1e-7;
                    p[3] = (pz * 0.999999 + e * 0.000001).clamp(-1e6, 1e6);
                }
                local_flops += 32 * 16;
            }
        }

        total_hashes.fetch_add(local_hashes, Ordering::Relaxed);
        total_flops.fetch_add(local_flops, Ordering::Relaxed);
        entropy_pool.fetch_xor(local_entropy ^ p[1].to_bits(), Ordering::Relaxed);

        if !sleep_budget.is_zero() {
            thread::sleep(sleep_budget);
        }
    }
}
