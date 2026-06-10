//! Cross-sample-rate regression tests.
//!
//! Every engine is rendered at several sample rates and compared against the
//! 48kHz reference rendering. The output must be perceptually identical:
//! pitch, spectral balance (energy in half-octave bands, mapped to absolute
//! Hz) and the loudness envelope (RMS trajectory in absolute time) must all
//! match within tight tolerances. Engines built on random processes (dust,
//! grains, noise) are compared statistically with looser tolerances.

use std::f64::consts::PI;
use std::sync::{Mutex, MutexGuard, OnceLock};

use mi_plaits_dsp::fx::diffuser::Diffuser;
use mi_plaits_dsp::fx::ensemble::Ensemble;
use mi_plaits_dsp::utils::random;
use mi_plaits_dsp::voice::{Modulations, Patch, Voice};

const BLOCK_SIZE: usize = 24;
const REF_SR: f32 = 48000.0;
const TEST_SRS: [f32; 3] = [32000.0, 44100.0, 96000.0];

/// The RNG is a global; serialize renders so each one sees a deterministic
/// random sequence even when the test harness runs tests on multiple threads.
fn render_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    match LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[derive(Clone)]
struct RenderConfig {
    engine: usize,
    triggered: bool,
    seconds: f32,
    note: f32,
    harmonics: f32,
    timbre: f32,
    morph: f32,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            engine: 0,
            triggered: false,
            seconds: 2.0,
            note: 48.0,
            harmonics: 0.5,
            timbre: 0.5,
            morph: 0.5,
        }
    }
}

fn render(config: &RenderConfig, sample_rate: f32) -> Vec<f32> {
    let _guard = render_lock();
    random::seed(0x1234_5678);

    let mut voice = Box::new(Voice::new(BLOCK_SIZE, sample_rate));
    voice.init();

    let patch = Patch {
        engine: config.engine,
        note: config.note,
        harmonics: config.harmonics,
        timbre: config.timbre,
        morph: config.morph,
        ..Default::default()
    };

    let mut modulations = Modulations {
        trigger_patched: config.triggered,
        ..Default::default()
    };

    let blocks = (config.seconds * sample_rate / BLOCK_SIZE as f32) as usize;
    let mut out = [0.0; BLOCK_SIZE];
    let mut aux = [0.0; BLOCK_SIZE];
    let mut result = Vec::with_capacity(blocks * BLOCK_SIZE);

    for block in 0..blocks {
        if config.triggered {
            // Trigger at 100ms (not at t=0) so envelope comparison windows
            // are not sensitive to the rounding of the trigger delay.
            let t = (block * BLOCK_SIZE) as f32 / sample_rate;
            modulations.trigger = if (0.1..0.2).contains(&t) { 1.0 } else { 0.0 };
        }
        voice.render(&patch, &modulations, &mut out, &mut aux);
        result.extend_from_slice(&out);
    }

    result
}

// ---------------------------------------------------------------------------
// Feature extraction
// ---------------------------------------------------------------------------

fn fft(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    assert!(n.is_power_of_two());

    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }

    let mut len = 2;
    while len <= n {
        let angle = -2.0 * PI / len as f64;
        let (w_im, w_re) = angle.sin_cos();
        for start in (0..n).step_by(len) {
            let mut cur_re = 1.0;
            let mut cur_im = 0.0;
            for k in 0..len / 2 {
                let a = start + k;
                let b = start + k + len / 2;
                let t_re = re[b] * cur_re - im[b] * cur_im;
                let t_im = re[b] * cur_im + im[b] * cur_re;
                re[b] = re[a] - t_re;
                im[b] = im[a] - t_im;
                re[a] += t_re;
                im[a] += t_im;
                let next_re = cur_re * w_re - cur_im * w_im;
                cur_im = cur_re * w_im + cur_im * w_re;
                cur_re = next_re;
            }
        }
        len <<= 1;
    }
}

/// Power spectrum of a Hann-windowed slice covering the same absolute time
/// range at any sample rate. Powers are normalized by the window length so
/// both tonal peaks and noise band powers are comparable across sample rates.
/// Returns (power per bin, bin width in Hz).
fn power_spectrum(
    signal: &[f32],
    sample_rate: f32,
    start_s: f32,
    duration_s: f32,
) -> (Vec<f64>, f64) {
    let start = ((start_s * sample_rate) as usize).min(signal.len());
    let len = ((duration_s * sample_rate) as usize).min(signal.len() - start);
    assert!(len > 0, "empty analysis window");

    let n_fft = len.next_power_of_two() * 2;
    let mut re = vec![0.0f64; n_fft];
    let mut im = vec![0.0f64; n_fft];

    for i in 0..len {
        let w = 0.5 - 0.5 * (2.0 * PI * i as f64 / len as f64).cos();
        re[i] = signal[start + i] as f64 * w;
    }

    fft(&mut re, &mut im);

    let norm = (len as f64) * (len as f64);
    let power = (0..n_fft / 2)
        .map(|k| (re[k] * re[k] + im[k] * im[k]) / norm)
        .collect();
    let bin_hz = sample_rate as f64 / n_fft as f64;

    (power, bin_hz)
}

/// Half-octave spaced filterbank center frequencies from 50Hz up to `f_max`.
fn band_centers(f_max: f64) -> Vec<f64> {
    let mut centers = vec![50.0];
    loop {
        let next = centers.last().unwrap() * 2.0f64.sqrt();
        if next > f_max {
            break;
        }
        centers.push(next);
    }
    centers
}

/// Triangular (in log-frequency) filterbank energies. Overlapping smooth
/// bands make the comparison insensitive to tones that sit exactly on a band
/// boundary, where rectangular bins would split leakage differently at
/// different FFT resolutions.
fn band_energies(power: &[f64], bin_hz: f64, centers: &[f64]) -> Vec<f64> {
    let mut energies = vec![0.0; centers.len().saturating_sub(2)];

    for (band, energy) in energies.iter_mut().enumerate() {
        let lo = centers[band].ln();
        let mid = centers[band + 1].ln();
        let hi = centers[band + 2].ln();

        let first_bin = ((centers[band] / bin_hz) as usize).max(1);
        let last_bin = ((centers[band + 2] / bin_hz) as usize).min(power.len());
        for k in first_bin..last_bin {
            let log_f = (k as f64 * bin_hz).ln();
            let weight = if log_f < mid {
                (log_f - lo) / (mid - lo)
            } else {
                (hi - log_f) / (hi - mid)
            };
            if weight > 0.0 {
                *energy += weight * power[k];
            }
        }
    }

    energies
}

fn db(x: f64) -> f64 {
    10.0 * (x + 1e-30).log10()
}

/// Compare band energies between a reference and another rendering. Bands
/// quieter than `floor_db` below the loudest band are skipped, the rest must
/// match within `tolerance_db`. Bands close to the lower Nyquist frequency
/// are excluded since anti-aliasing legitimately differs there.
fn assert_bands_match(
    reference: &[f32],
    other: &[f32],
    ref_sr: f32,
    other_sr: f32,
    start_s: f32,
    duration_s: f32,
    tolerance_db: f64,
    label: &str,
) {
    let f_max = 0.18 * ref_sr.min(other_sr) as f64;
    let centers = band_centers(f_max);

    let (ref_power, ref_bin) = power_spectrum(reference, ref_sr, start_s, duration_s);
    let (other_power, other_bin) = power_spectrum(other, other_sr, start_s, duration_s);

    let ref_bands = band_energies(&ref_power, ref_bin, &centers);
    let other_bands = band_energies(&other_power, other_bin, &centers);

    let max_band = ref_bands.iter().cloned().fold(f64::MIN, f64::max);
    let floor_db = -35.0;

    for (i, (a, b)) in ref_bands.iter().zip(other_bands.iter()).enumerate() {
        if db(*a) < db(max_band) + floor_db {
            continue;
        }
        let diff = (db(*b) - db(*a)).abs();
        assert!(
            diff < tolerance_db,
            "{}: band at {:.0}Hz differs by {:.2}dB (tolerance {:.2}dB) at {}Hz vs {}Hz",
            label,
            centers[i + 1],
            diff,
            tolerance_db,
            other_sr,
            ref_sr
        );
    }
}

/// RMS envelope in consecutive windows of `window_s` seconds, in dB.
fn rms_envelope_db(signal: &[f32], sample_rate: f32, window_s: f32) -> Vec<f64> {
    let window = (window_s * sample_rate) as usize;
    signal
        .chunks(window)
        .filter(|chunk| chunk.len() == window)
        .map(|chunk| {
            let power: f64 = chunk.iter().map(|x| (*x as f64) * (*x as f64)).sum();
            db(power / chunk.len() as f64)
        })
        .collect()
}

/// Compare RMS envelopes in absolute time. Windows quieter than `floor_db`
/// below the loudest window are skipped; for the rest, the other envelope
/// must match within `tolerance_db`, allowing one window of time skew.
fn assert_envelopes_match(
    reference: &[f32],
    other: &[f32],
    ref_sr: f32,
    other_sr: f32,
    window_s: f32,
    tolerance_db: f64,
    label: &str,
) {
    let ref_env = rms_envelope_db(reference, ref_sr, window_s);
    let other_env = rms_envelope_db(other, other_sr, window_s);
    let len = ref_env.len().min(other_env.len());

    let max_db = ref_env.iter().cloned().fold(f64::MIN, f64::max);
    let floor_db = -40.0;

    for i in 0..len {
        if ref_env[i] < max_db + floor_db {
            continue;
        }
        let lo = i.saturating_sub(1);
        let hi = (i + 1).min(len - 1);
        let diff = (lo..=hi)
            .map(|j| (other_env[j] - ref_env[i]).abs())
            .fold(f64::MAX, f64::min);
        assert!(
            diff < tolerance_db,
            "{}: envelope at {:.0}ms differs by {:.2}dB (tolerance {:.2}dB) at {}Hz vs {}Hz",
            label,
            i as f32 * window_s * 1000.0,
            diff,
            tolerance_db,
            other_sr,
            ref_sr
        );
    }
}

/// Compare per-band energy decay rates between two time windows. The string
/// and modal exciters are random noise bursts, so absolute band levels vary
/// between realizations; the decay rate of each band is the realization
/// independent quantity that captures damping behavior.
#[allow(clippy::too_many_arguments)]
fn assert_band_decay_match(
    reference: &[f32],
    other: &[f32],
    ref_sr: f32,
    other_sr: f32,
    early: (f32, f32),
    late: (f32, f32),
    tolerance_db: f64,
    label: &str,
) {
    let f_max = 0.18 * ref_sr.min(other_sr) as f64;
    let centers = band_centers(f_max);

    let bands = |signal: &[f32], sr: f32, window: (f32, f32)| {
        let (power, bin_hz) = power_spectrum(signal, sr, window.0, window.1);
        band_energies(&power, bin_hz, &centers)
    };

    let ref_early = bands(reference, ref_sr, early);
    let ref_late = bands(reference, ref_sr, late);
    let other_early = bands(other, other_sr, early);
    let other_late = bands(other, other_sr, late);

    let max_band = ref_early.iter().cloned().fold(f64::MIN, f64::max);
    let floor_db = -45.0;

    for i in 0..ref_early.len() {
        if db(ref_early[i]) < db(max_band) + floor_db {
            continue;
        }
        let ref_decay = db(ref_early[i]) - db(ref_late[i]);
        let other_decay = db(other_early[i]) - db(other_late[i]);
        let diff = (ref_decay - other_decay).abs();
        assert!(
            diff < tolerance_db,
            "{}: band at {:.0}Hz decays by {:.1}dB at {}Hz vs {:.1}dB at {}Hz (tolerance {:.1}dB)",
            label,
            centers[i + 1],
            other_decay,
            other_sr,
            ref_decay,
            ref_sr,
            tolerance_db
        );
    }
}

/// Fundamental frequency estimate via normalized autocorrelation on a slice
/// in the middle of the signal.
fn pitch_hz(signal: &[f32], sample_rate: f32) -> f64 {
    let window = (0.2 * sample_rate) as usize;
    let start = signal.len() / 2;
    let slice: Vec<f64> = signal[start..(start + window).min(signal.len())]
        .iter()
        .map(|x| *x as f64)
        .collect();

    let min_lag = (sample_rate / 1500.0) as usize;
    let max_lag = (sample_rate / 35.0) as usize;
    let n = slice.len() - max_lag;

    let energy: f64 = slice[..n].iter().map(|x| x * x).sum();

    let mut correlations = Vec::with_capacity(max_lag - min_lag);
    for lag in min_lag..max_lag {
        let mut corr = 0.0;
        let mut lag_energy = 0.0;
        for i in 0..n {
            corr += slice[i] * slice[i + lag];
            lag_energy += slice[i + lag] * slice[i + lag];
        }
        correlations.push(corr / (energy * lag_energy + 1e-30).sqrt());
    }

    // The autocorrelation starts near 1.0 for small lags; skip this initial
    // plateau before searching for the periodicity peak.
    let mut search_start = 0;
    while search_start < correlations.len() && correlations[search_start] > 0.5 {
        search_start += 1;
    }
    if search_start >= correlations.len() {
        search_start = 0;
    }

    let max_corr = correlations[search_start..]
        .iter()
        .cloned()
        .fold(f64::MIN, f64::max);
    // Among local maxima close to the global maximum, pick the SHORTEST lag
    // to avoid sub-octave errors.
    let threshold = 0.9 * max_corr;
    let mut best = None;
    for i in search_start.max(1)..correlations.len() - 1 {
        if correlations[i] >= threshold
            && correlations[i] >= correlations[i - 1]
            && correlations[i] >= correlations[i + 1]
        {
            best = Some(i);
            break;
        }
    }
    let best = best.unwrap_or_else(|| {
        correlations[search_start..]
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i + search_start)
            .unwrap()
    });

    // Parabolic refinement.
    let mut lag = (min_lag + best) as f64;
    if best > 0 && best + 1 < correlations.len() {
        let (a, b, c) = (
            correlations[best - 1],
            correlations[best],
            correlations[best + 1],
        );
        let denominator = a - 2.0 * b + c;
        if denominator.abs() > 1e-12 {
            lag += 0.5 * (a - c) / denominator;
        }
    }

    sample_rate as f64 / lag
}

fn assert_pitch_matches(
    reference: &[f32],
    other: &[f32],
    ref_sr: f32,
    other_sr: f32,
    tolerance_cents: f64,
    label: &str,
) {
    let ref_pitch = pitch_hz(reference, ref_sr);
    let other_pitch = pitch_hz(other, other_sr);
    let cents = 1200.0 * (other_pitch / ref_pitch).log2().abs();
    assert!(
        cents < tolerance_cents,
        "{}: pitch {:.2}Hz at {}Hz vs {:.2}Hz at {}Hz differs by {:.1} cents (tolerance {:.1})",
        label,
        other_pitch,
        other_sr,
        ref_pitch,
        ref_sr,
        cents,
        tolerance_cents
    );
}

fn total_rms_db(signal: &[f32]) -> f64 {
    let power: f64 = signal.iter().map(|x| (*x as f64) * (*x as f64)).sum();
    db(power / signal.len() as f64)
}

fn spectral_centroid_hz(signal: &[f32], sample_rate: f32, f_max: f64) -> f64 {
    let duration = (signal.len() as f32 / sample_rate).min(2.0);
    let (power, bin_hz) = power_spectrum(signal, sample_rate, 0.0, duration);
    let max_bin = ((f_max / bin_hz) as usize).min(power.len());
    let mut weighted = 0.0;
    let mut total = 0.0;
    for (k, p) in power[..max_bin].iter().enumerate() {
        weighted += k as f64 * bin_hz * p;
        total += p;
    }
    weighted / (total + 1e-30)
}

// ---------------------------------------------------------------------------
// Per-engine checks
// ---------------------------------------------------------------------------

struct Checks {
    pitch_cents: Option<f64>,
    bands_db: Option<f64>,
    envelope_db: Option<f64>,
    centroid_ratio: Option<f64>,
    // Analysis window for the band comparison.
    band_window: (f32, f32),
}

impl Default for Checks {
    fn default() -> Self {
        Self {
            pitch_cents: Some(10.0),
            bands_db: Some(1.5),
            envelope_db: None,
            centroid_ratio: None,
            band_window: (0.25, 1.5),
        }
    }
}

fn check_engine(config: &RenderConfig, checks: &Checks, label: &str) {
    let reference = render(config, REF_SR);
    assert!(
        total_rms_db(&reference) > -60.0,
        "{}: reference render is silent",
        label
    );

    for sr in TEST_SRS {
        let other = render(config, sr);

        if let Some(tolerance) = checks.pitch_cents {
            assert_pitch_matches(&reference, &other, REF_SR, sr, tolerance, label);
        }
        if let Some(tolerance) = checks.bands_db {
            let (start, duration) = checks.band_window;
            assert_bands_match(
                &reference, &other, REF_SR, sr, start, duration, tolerance, label,
            );
        }
        if let Some(tolerance) = checks.envelope_db {
            assert_envelopes_match(&reference, &other, REF_SR, sr, 0.02, tolerance, label);
        }
        if let Some(tolerance) = checks.centroid_ratio {
            let f_max = 0.18 * REF_SR.min(sr) as f64;
            let ref_centroid = spectral_centroid_hz(&reference, REF_SR, f_max);
            let other_centroid = spectral_centroid_hz(&other, sr, f_max);
            let ratio = (other_centroid / ref_centroid).max(ref_centroid / other_centroid);
            assert!(
                ratio < tolerance,
                "{}: spectral centroid {:.0}Hz at {}Hz vs {:.0}Hz at {}Hz (tolerance ratio {:.2})",
                label,
                other_centroid,
                sr,
                ref_centroid,
                REF_SR,
                tolerance
            );
        }
    }
}

#[test]
fn virtual_analog_vcf_engine() {
    check_engine(
        &RenderConfig {
            engine: 0,
            ..Default::default()
        },
        &Checks::default(),
        "virtual_analog_vcf",
    );
}

#[test]
fn phase_distortion_engine() {
    check_engine(
        &RenderConfig {
            engine: 1,
            ..Default::default()
        },
        &Checks::default(),
        "phase_distortion",
    );
}

#[test]
fn six_op_engine() {
    check_engine(
        &RenderConfig {
            engine: 2,
            triggered: true,
            ..Default::default()
        },
        &Checks {
            pitch_cents: Some(10.0),
            bands_db: Some(2.0),
            envelope_db: Some(2.5),
            centroid_ratio: None,
            band_window: (0.15, 1.0),
        },
        "six_op",
    );
}

#[test]
fn wave_terrain_engine() {
    check_engine(
        &RenderConfig {
            engine: 5,
            ..Default::default()
        },
        &Checks::default(),
        "wave_terrain",
    );
}

#[test]
fn string_machine_engine() {
    check_engine(
        &RenderConfig {
            engine: 6,
            seconds: 3.0,
            ..Default::default()
        },
        &Checks {
            pitch_cents: None,
            bands_db: Some(2.0),
            band_window: (0.25, 2.5),
            ..Default::default()
        },
        "string_machine",
    );
}

#[test]
fn chiptune_engine() {
    check_engine(
        &RenderConfig {
            engine: 7,
            ..Default::default()
        },
        &Checks {
            pitch_cents: None,
            ..Default::default()
        },
        "chiptune",
    );
}

#[test]
fn virtual_analog_engine() {
    check_engine(
        &RenderConfig {
            engine: 8,
            ..Default::default()
        },
        &Checks::default(),
        "virtual_analog",
    );
}

#[test]
fn waveshaping_engine() {
    check_engine(
        &RenderConfig {
            engine: 9,
            ..Default::default()
        },
        &Checks::default(),
        "waveshaping",
    );
}

#[test]
fn fm_engine() {
    check_engine(
        &RenderConfig {
            engine: 10,
            ..Default::default()
        },
        &Checks::default(),
        "fm",
    );
}

#[test]
fn grain_engine() {
    check_engine(
        &RenderConfig {
            engine: 11,
            ..Default::default()
        },
        &Checks::default(),
        "grain",
    );
}

#[test]
fn additive_engine() {
    check_engine(
        &RenderConfig {
            engine: 12,
            ..Default::default()
        },
        &Checks::default(),
        "additive",
    );
}

#[test]
fn wavetable_engine() {
    check_engine(
        &RenderConfig {
            engine: 13,
            ..Default::default()
        },
        &Checks::default(),
        "wavetable",
    );
}

#[test]
fn chord_engine() {
    check_engine(
        &RenderConfig {
            engine: 14,
            ..Default::default()
        },
        &Checks {
            pitch_cents: None,
            ..Default::default()
        },
        "chord",
    );
}

#[test]
fn speech_engine_vowels() {
    check_engine(
        &RenderConfig {
            engine: 15,
            harmonics: 0.25,
            ..Default::default()
        },
        &Checks {
            pitch_cents: Some(15.0),
            bands_db: Some(2.0),
            ..Default::default()
        },
        "speech_vowels",
    );
}

#[test]
fn speech_engine_words() {
    check_engine(
        &RenderConfig {
            engine: 15,
            harmonics: 0.66,
            seconds: 3.0,
            ..Default::default()
        },
        &Checks {
            pitch_cents: None,
            bands_db: Some(2.5),
            envelope_db: Some(3.0),
            centroid_ratio: None,
            band_window: (0.0, 3.0),
        },
        "speech_words",
    );
}

#[test]
fn swarm_engine() {
    check_engine(
        &RenderConfig {
            engine: 16,
            seconds: 4.0,
            ..Default::default()
        },
        &Checks {
            pitch_cents: None,
            bands_db: Some(3.0),
            band_window: (0.25, 3.5),
            ..Default::default()
        },
        "swarm",
    );
}

#[test]
fn noise_engine() {
    check_engine(
        &RenderConfig {
            engine: 17,
            seconds: 4.0,
            harmonics: 0.25,
            ..Default::default()
        },
        &Checks {
            pitch_cents: None,
            bands_db: Some(2.5),
            band_window: (0.25, 3.5),
            ..Default::default()
        },
        "noise",
    );
}

#[test]
fn particle_engine() {
    // The particle engine randomizes impulse times and filter frequencies,
    // so even long renders have high band-level variance. Compare overall
    // level and spectral centroid instead.
    let config = RenderConfig {
        engine: 18,
        seconds: 6.0,
        ..Default::default()
    };
    let reference = render(&config, REF_SR);
    let ref_rms = total_rms_db(&reference);
    assert!(ref_rms > -60.0, "particle: reference render is silent");

    for sr in TEST_SRS {
        let other = render(&config, sr);
        let rms_diff = (total_rms_db(&other) - ref_rms).abs();
        assert!(
            rms_diff < 3.0,
            "particle: overall RMS differs by {:.2}dB at {}Hz",
            rms_diff,
            sr
        );

        let f_max = 0.18 * REF_SR.min(sr) as f64;
        let ref_centroid = spectral_centroid_hz(&reference, REF_SR, f_max);
        let other_centroid = spectral_centroid_hz(&other, sr, f_max);
        let ratio = (other_centroid / ref_centroid).max(ref_centroid / other_centroid);
        assert!(
            ratio < 1.35,
            "particle: spectral centroid {:.0}Hz vs {:.0}Hz at {}Hz",
            other_centroid,
            ref_centroid,
            sr
        );
    }
}

#[test]
fn string_engine_plucked() {
    string_pluck_test(48.0, "string_plucked");
}

#[test]
fn string_engine_low_note() {
    // Exercises the longer Karplus-Strong delays: at 96kHz this note only
    // fits because the delay line is scaled with the sample rate.
    string_pluck_test(31.0, "string_low_note");
}

/// The string exciter is a random noise burst, so band levels differ between
/// realizations at different sample rates. Compare the realization
/// independent features: pitch and per-band decay rates.
fn string_pluck_test(note: f32, label: &str) {
    // Half-octave bands contain multiple harmonics for low notes; their
    // composite decay rate depends on the random excitation balance.
    let decay_tolerance = if note < 40.0 { 10.0 } else { 6.0 };
    let config = RenderConfig {
        engine: 19,
        triggered: true,
        note,
        ..Default::default()
    };
    let reference = render(&config, REF_SR);
    assert!(
        total_rms_db(&reference) > -60.0,
        "{}: reference render is silent",
        label
    );

    for sr in TEST_SRS {
        let other = render(&config, sr);
        assert_pitch_matches(&reference, &other, REF_SR, sr, 10.0, label);
        assert_band_decay_match(
            &reference,
            &other,
            REF_SR,
            sr,
            (0.15, 0.4),
            (0.95, 0.4),
            decay_tolerance,
            label,
        );
    }
}

#[test]
fn string_engine_sustained_dust() {
    let config = RenderConfig {
        engine: 19,
        seconds: 6.0,
        ..Default::default()
    };
    let reference = render(&config, REF_SR);
    let ref_rms = total_rms_db(&reference);
    assert!(ref_rms > -60.0, "string_dust: reference render is silent");

    for sr in TEST_SRS {
        let other = render(&config, sr);
        let rms_diff = (total_rms_db(&other) - ref_rms).abs();
        assert!(
            rms_diff < 3.0,
            "string_dust: overall RMS differs by {:.2}dB at {}Hz",
            rms_diff,
            sr
        );
        // No pitch check: the dust excitation is a random process and the
        // dispersive string makes single-realization pitch estimates unstable.
        // Tuning is covered deterministically by `string_engine_plucked`.
    }
}

#[test]
fn modal_engine_struck() {
    check_engine(
        &RenderConfig {
            engine: 20,
            triggered: true,
            ..Default::default()
        },
        &Checks {
            pitch_cents: Some(10.0),
            bands_db: Some(2.5),
            envelope_db: Some(3.0),
            centroid_ratio: None,
            band_window: (0.15, 1.3),
        },
        "modal_struck",
    );
}

#[test]
fn modal_engine_sustained_dust() {
    let config = RenderConfig {
        engine: 20,
        seconds: 6.0,
        ..Default::default()
    };
    let reference = render(&config, REF_SR);
    let ref_rms = total_rms_db(&reference);
    assert!(ref_rms > -60.0, "modal_dust: reference render is silent");

    for sr in TEST_SRS {
        let other = render(&config, sr);
        let rms_diff = (total_rms_db(&other) - ref_rms).abs();
        assert!(
            rms_diff < 3.0,
            "modal_dust: overall RMS differs by {:.2}dB at {}Hz",
            rms_diff,
            sr
        );
    }
}

#[test]
fn bass_drum_engine_analog() {
    check_engine(
        &RenderConfig {
            engine: 21,
            triggered: true,
            seconds: 1.5,
            harmonics: 0.2,
            ..Default::default()
        },
        &Checks {
            pitch_cents: Some(15.0),
            bands_db: Some(2.0),
            envelope_db: Some(2.5),
            centroid_ratio: None,
            band_window: (0.1, 1.0),
        },
        "bass_drum_analog",
    );
}

#[test]
fn bass_drum_engine_synthetic() {
    check_engine(
        &RenderConfig {
            engine: 21,
            triggered: true,
            seconds: 1.5,
            harmonics: 0.8,
            ..Default::default()
        },
        &Checks {
            pitch_cents: Some(15.0),
            bands_db: Some(2.0),
            envelope_db: Some(2.5),
            centroid_ratio: None,
            band_window: (0.1, 1.0),
        },
        "bass_drum_synthetic",
    );
}

#[test]
fn snare_drum_engine_analog() {
    check_engine(
        &RenderConfig {
            engine: 22,
            triggered: true,
            seconds: 1.5,
            harmonics: 0.2,
            ..Default::default()
        },
        &Checks {
            pitch_cents: None,
            bands_db: Some(3.0),
            envelope_db: Some(3.0),
            centroid_ratio: None,
            band_window: (0.1, 1.0),
        },
        "snare_drum_analog",
    );
}

#[test]
fn snare_drum_engine_synthetic() {
    check_engine(
        &RenderConfig {
            engine: 22,
            triggered: true,
            seconds: 1.5,
            harmonics: 0.8,
            ..Default::default()
        },
        &Checks {
            pitch_cents: None,
            bands_db: Some(3.0),
            envelope_db: Some(3.0),
            centroid_ratio: None,
            band_window: (0.1, 1.0),
        },
        "snare_drum_synthetic",
    );
}

#[test]
fn hihat_engine() {
    check_engine(
        &RenderConfig {
            engine: 23,
            triggered: true,
            seconds: 1.5,
            ..Default::default()
        },
        &Checks {
            pitch_cents: None,
            bands_db: Some(3.0),
            envelope_db: Some(3.0),
            centroid_ratio: None,
            band_window: (0.1, 1.0),
        },
        "hihat",
    );
}

#[test]
fn lpg_envelope_and_filter() {
    // A triggered melodic engine exercises the internal decay envelope, the
    // vactrol simulation and the low pass gate filter color.
    check_engine(
        &RenderConfig {
            engine: 8,
            triggered: true,
            ..Default::default()
        },
        &Checks {
            pitch_cents: Some(10.0),
            bands_db: Some(2.0),
            envelope_db: Some(2.5),
            centroid_ratio: None,
            band_window: (0.1, 1.0),
        },
        "lpg",
    );
}

// ---------------------------------------------------------------------------
// Component-level tests
// ---------------------------------------------------------------------------

#[test]
fn diffuser_decay() {
    // Feed an impulse and verify the reverb tail decays identically in
    // absolute time at any sample rate.
    fn render_diffuser(sample_rate: f32) -> Vec<f32> {
        let _guard = render_lock();
        let mut diffuser = Diffuser::new();
        diffuser.init(sample_rate);
        diffuser.clear();

        // Past ~1s the tail reaches the fixed quantization floor of the
        // 12-bit delay line storage (>35dB below the wet peak), which is
        // inherently not sample rate invariant; stop the comparison before.
        let total = (1.0 * sample_rate) as usize;
        let mut signal = vec![0.0f32; total];
        // Constant-area impulse: a one-sample impulse represents the same
        // continuous-time impulse only when its amplitude scales with the
        // sample rate.
        signal[0] = sample_rate / REF_SR;
        for chunk in signal.chunks_mut(BLOCK_SIZE) {
            diffuser.process(1.0, 0.8, chunk);
        }
        signal
    }

    let reference = render_diffuser(REF_SR);
    assert!(
        total_rms_db(&reference) > -60.0,
        "diffuser output is silent"
    );

    for sr in TEST_SRS {
        let other = render_diffuser(sr);
        assert_envelopes_match(&reference, &other, REF_SR, sr, 0.1, 3.0, "diffuser");
    }
}

#[test]
fn ensemble_modulation() {
    // The chorus interference pattern follows the two LFOs; the resulting
    // amplitude modulation must evolve identically in absolute time.
    fn render_ensemble(sample_rate: f32) -> Vec<f32> {
        let _guard = render_lock();
        let mut ensemble = Ensemble::new();
        ensemble.init(sample_rate);
        ensemble.clear();
        ensemble.set_amount(1.0);
        ensemble.set_depth(1.0);

        let total = (4.0 * sample_rate) as usize;
        let f = 220.0 / sample_rate;
        let mut phase = 0.0f32;
        let mut left = vec![0.0f32; total];
        for sample in left.iter_mut() {
            phase += f;
            if phase >= 1.0 {
                phase -= 1.0;
            }
            *sample = (2.0 * core::f32::consts::PI * phase).sin() * 0.5;
        }
        let mut right = left.clone();
        for (left_chunk, right_chunk) in left
            .chunks_mut(BLOCK_SIZE)
            .zip(right.chunks_mut(BLOCK_SIZE))
        {
            ensemble.process(left_chunk, right_chunk);
        }
        left
    }

    let reference = render_ensemble(REF_SR);
    for sr in TEST_SRS {
        let other = render_ensemble(sr);
        assert_envelopes_match(&reference, &other, REF_SR, sr, 0.05, 1.5, "ensemble");
    }
}
// Temporary diagnostic appended to the test file
